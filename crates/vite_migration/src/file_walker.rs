use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use vite_error::Error;

// TODO: only support esm files for now
/// File extensions to process for import rewriting
const TS_JS_EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "js", "jsx", "mjs"];

/// Result of walking TypeScript/JavaScript files
#[derive(Debug)]
pub struct WalkResult {
    /// List of file paths found
    pub files: Vec<PathBuf>,
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
/// use vite_migration::find_ts_files;
///
/// let result = find_ts_files(Path::new("./src"))?;
/// for file in result.files {
///     println!("Found: {}", file.display());
/// }
/// ```
pub fn find_ts_files(root: &Path) -> Result<WalkResult, Error> {
    let mut files = Vec::new();

    let walker = WalkBuilder::new(root)
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
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if TS_JS_EXTENSIONS.contains(&ext) {
                files.push(path.to_path_buf());
            }
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

        assert_eq!(result.files.len(), 6);
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
