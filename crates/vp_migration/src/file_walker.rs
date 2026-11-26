use std::path::{Path, PathBuf};

use ignore::{
    WalkBuilder,
    gitignore::{Gitignore, GitignoreBuilder},
};
use vp_error::Error;

/// File extensions to process for import rewriting.
///
/// Includes CommonJS (`.cjs`/`.cts`) variants in addition to ESM, because
/// the legacy reverse-migration rules also cover `require('vite-plus/test/browser-*')`
/// calls that are most commonly found in `.cjs` / `vitest.config.cjs` files.
const TS_JS_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

/// Result of walking TypeScript/JavaScript files
#[derive(Debug)]
pub struct WalkResult {
    /// List of file paths found
    pub files: Vec<PathBuf>,
}

/// Check a possibly nonexistent directory against repository-owned `.gitignore`
/// files from `root` down to its parent. Do not use machine-local Git excludes:
/// migration must produce ignore rules that also work for other contributors.
pub fn is_directory_gitignored(root: &Path, directory: &Path) -> Result<bool, Error> {
    let relative = directory
        .strip_prefix(root)
        .map_err(|_| Error::InvalidArgument("Ignore target must be inside the root".into()))?;
    match (directory.parent(), directory.file_name()) {
        (Some(parent), Some(name)) if relative.components().next().is_some() => {
            Ok(gitignored_children(root, parent, &[name])?[0])
        }
        _ => Ok(false),
    }
}

/// Return the child directory names of `parent` that are not gitignored. Uses the
/// same rules as [`is_directory_gitignored`], but reads the ignore files once.
pub fn unignored_child_directories(
    root: &Path,
    parent: &Path,
    names: Vec<String>,
) -> Result<Vec<String>, Error> {
    if names.is_empty() {
        return Ok(names);
    }
    let ignored = gitignored_children(root, parent, &names)?;
    Ok(names
        .into_iter()
        .zip(ignored)
        .filter(|(_, ignored)| !ignored)
        .map(|(name, _)| name)
        .collect())
}

fn gitignored_children(
    root: &Path,
    parent: &Path,
    names: &[impl AsRef<Path>],
) -> Result<Vec<bool>, Error> {
    let relative = parent
        .strip_prefix(root)
        .map_err(|_| Error::InvalidArgument("Ignore target must be inside the root".into()))?;
    let mut current = root.to_path_buf();
    let mut matchers = Vec::new();
    for component in relative.components() {
        add_ignore_file(&current, &mut matchers)?;
        current.push(component);
        // Check each directory before descending: a negation cannot
        // re-include an ignored parent directory.
        if is_ignored(&matchers, &current) {
            return Ok(vec![true; names.len()]);
        }
    }
    add_ignore_file(&current, &mut matchers)?;
    Ok(names.iter().map(|name| is_ignored(&matchers, &current.join(name))).collect())
}

fn add_ignore_file(directory: &Path, matchers: &mut Vec<Gitignore>) -> Result<(), Error> {
    let ignore_file = directory.join(".gitignore");
    if ignore_file.is_file() {
        let mut builder = GitignoreBuilder::new(directory);
        let _ = builder.add(&ignore_file);
        matchers.push(builder.build()?);
    }
    Ok(())
}

/// Deeper ignore files take precedence.
fn is_ignored(matchers: &[Gitignore], directory: &Path) -> bool {
    matchers
        .iter()
        .rev()
        .map(|matcher| matcher.matched(directory, true))
        .find(|matched| !matched.is_none())
        .is_some_and(|matched| matched.is_ignore())
}

/// Find all TypeScript/JavaScript files in a directory, respecting gitignore
///
/// This function walks the directory tree starting from `root` and finds all files
/// with TypeScript or JavaScript extensions (.ts, .tsx, .mts, .cts, .js, .jsx, .mjs, .cjs).
///
/// The walk respects:
/// - `.gitignore` files in the directory tree
/// - Global gitignore configuration
/// - `.git/info/exclude` files
/// - Hidden files and directories are skipped
///
/// # Arguments
///
/// * `root` - The root directory to start searching from
///
/// # Returns
///
/// Returns a `WalkResult` containing the list of found files, or an error if
/// the directory walk fails.
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use vp_migration::find_ts_files;
///
/// let result = find_ts_files(Path::new("./src"))?;
/// for file in result.files {
///     println!("Found: {}", file.display());
/// }
/// ```
pub fn find_ts_files(root: &Path) -> Result<WalkResult, Error> {
    find_ts_files_filtered(root, |_| true)
}

pub(crate) fn find_ts_files_filtered(
    root: &Path,
    include: impl Fn(&ignore::DirEntry) -> bool + Send + Sync + 'static,
) -> Result<WalkResult, Error> {
    let mut files = Vec::new();

    let walker = WalkBuilder::new(root)
        .filter_entry(include)
        .hidden(true) // Skip hidden files/dirs
        .git_ignore(true) // Respect .gitignore
        .git_global(true) // Respect global gitignore
        .git_exclude(true) // Respect .git/info/exclude
        .require_git(false) // Work even if not a git repo
        .build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Check extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && TS_JS_EXTENSIONS.contains(&ext)
        {
            files.push(path.to_path_buf());
        }
    }

    Ok(WalkResult { files })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_inherited_directory_gitignore() {
        for (root_rules, package_rules, expected) in [
            (".vitest/\n", "", true),
            (".vitest\n", "", true),
            ("**/.vitest/\n", "", true),
            ("/.vitest/\n", "", false),
            ("/packages/unit/.vitest/\n", "", true),
            (".vitest-reports/\n", "", false),
            ("# .vitest/\n", "", false),
            ("\\!.vitest/\n", "", false),
            (".vitest/\n!/packages/unit/.vitest/\n", "", false),
            (".vitest/\n", "!/.vitest/\n", false),
            ("!**/.vitest/\n", "/.vitest/\n", true),
            ("packages/\n", "!/.vitest/\n", true),
            ("packages/*\n!packages/unit/\n", "", false),
        ] {
            let temp = tempdir().unwrap();
            let package = temp.path().join("packages/unit");
            fs::create_dir_all(&package).unwrap();
            fs::write(temp.path().join(".gitignore"), root_rules).unwrap();
            fs::write(package.join(".gitignore"), package_rules).unwrap();
            // Neither a Git repository nor the target directory needs to exist.
            assert_eq!(
                is_directory_gitignored(temp.path(), &package.join(".vitest")).unwrap(),
                expected,
                "root: {root_rules:?}, package: {package_rules:?}"
            );
            assert_eq!(
                unignored_child_directories(temp.path(), &package, vec![".vitest".into()])
                    .unwrap()
                    .is_empty(),
                expected,
                "batch root: {root_rules:?}, package: {package_rules:?}"
            );
        }
    }

    #[test]
    fn test_invalid_gitignore_lines_are_skipped() {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join(".gitignore"), "{a,b\n.vitest/\n").unwrap();
        assert!(is_directory_gitignored(temp.path(), &temp.path().join(".vitest")).unwrap());
        assert_eq!(
            unignored_child_directories(
                temp.path(),
                temp.path(),
                vec![".vitest".into(), "src".into()]
            )
            .unwrap(),
            vec!["src".to_string()]
        );
    }

    #[test]
    fn test_directory_gitignore_boundary() {
        let temp = tempdir().unwrap();
        let package = temp.path().join("packages/unit");
        fs::create_dir_all(&package).unwrap();
        fs::write(temp.path().join(".gitignore"), ".vitest/\n").unwrap();
        assert!(!is_directory_gitignored(&package, &package.join(".vitest")).unwrap());
        assert!(is_directory_gitignored(&package, &temp.path().join(".vitest")).is_err());
    }

    #[test]
    fn test_find_ts_files_basic() {
        let temp = tempdir().unwrap();

        // Create test files
        fs::write(temp.path().join("app.ts"), "").unwrap();
        fs::write(temp.path().join("utils.tsx"), "").unwrap();
        fs::write(temp.path().join("config.js"), "").unwrap();
        fs::write(temp.path().join("readme.md"), "").unwrap();

        let result = find_ts_files(temp.path()).unwrap();

        // Should find ts, tsx, js but not md
        assert_eq!(result.files.len(), 3);
    }

    #[test]
    fn test_find_ts_files_nested() {
        let temp = tempdir().unwrap();

        // Create nested directory
        fs::create_dir(temp.path().join("src")).unwrap();
        fs::write(temp.path().join("src/index.ts"), "").unwrap();
        fs::write(temp.path().join("src/utils.tsx"), "").unwrap();

        // Create deeper nesting
        fs::create_dir_all(temp.path().join("src/components")).unwrap();
        fs::write(temp.path().join("src/components/Button.tsx"), "").unwrap();

        let result = find_ts_files(temp.path()).unwrap();

        assert_eq!(result.files.len(), 3);
    }

    #[test]
    fn test_find_ts_files_respects_gitignore() {
        let temp = tempdir().unwrap();

        // Create test files
        fs::write(temp.path().join("app.ts"), "").unwrap();

        // Create node_modules (should be ignored via gitignore)
        fs::create_dir(temp.path().join("node_modules")).unwrap();
        fs::write(temp.path().join("node_modules/pkg.ts"), "").unwrap();

        // Create dist (should be ignored via gitignore)
        fs::create_dir(temp.path().join("dist")).unwrap();
        fs::write(temp.path().join("dist/bundle.js"), "").unwrap();

        // Create .gitignore
        fs::write(temp.path().join(".gitignore"), "node_modules/\ndist/").unwrap();

        let result = find_ts_files(temp.path()).unwrap();

        // Should only find app.ts, not node_modules or dist files
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("app.ts"));
    }

    #[test]
    fn test_find_ts_files_all_extensions() {
        let temp = tempdir().unwrap();

        // Create files with all supported extensions
        fs::write(temp.path().join("a.ts"), "").unwrap();
        fs::write(temp.path().join("b.tsx"), "").unwrap();
        fs::write(temp.path().join("c.mts"), "").unwrap();
        fs::write(temp.path().join("d.cts"), "").unwrap();
        fs::write(temp.path().join("e.js"), "").unwrap();
        fs::write(temp.path().join("f.jsx"), "").unwrap();
        fs::write(temp.path().join("g.mjs"), "").unwrap();
        fs::write(temp.path().join("h.cjs"), "").unwrap();

        // Create non-matching files
        fs::write(temp.path().join("i.json"), "").unwrap();
        fs::write(temp.path().join("j.css"), "").unwrap();
        fs::write(temp.path().join("k.html"), "").unwrap();

        let result = find_ts_files(temp.path()).unwrap();

        assert_eq!(result.files.len(), 8);
    }

    #[test]
    fn test_find_ts_files_empty_directory() {
        let temp = tempdir().unwrap();

        let result = find_ts_files(temp.path()).unwrap();

        assert!(result.files.is_empty());
    }

    #[test]
    fn test_find_ts_files_skips_hidden() {
        let temp = tempdir().unwrap();

        // Create visible file
        fs::write(temp.path().join("visible.ts"), "").unwrap();

        // Create hidden directory with ts file
        fs::create_dir(temp.path().join(".hidden")).unwrap();
        fs::write(temp.path().join(".hidden/secret.ts"), "").unwrap();

        let result = find_ts_files(temp.path()).unwrap();

        // Should only find visible.ts
        assert_eq!(result.files.len(), 1);
        assert!(result.files[0].ends_with("visible.ts"));
    }
}
