use std::collections::{BTreeMap, HashSet, VecDeque};

use petgraph::Direction;
use rustc_hash::{FxHashMap, FxHashSet};
use vite_path::AbsolutePath;

/// Parsed package selector for `--filter` flag (pnpm-compatible syntax).
#[derive(Debug, Clone)]
pub(super) struct PackageSelector {
    pub name_pattern: Option<String>,
    pub parent_dir: Option<String>,
    pub include_dependencies: bool,
    pub include_dependents: bool,
    pub exclude_self: bool,
    pub exclude: bool,
}

/// Check if a selector string is a path selector (starts with `.` or `..`).
/// Matches pnpm's `isSelectorByLocation` logic.
fn is_path_selector(s: &str) -> bool {
    if !s.starts_with('.') {
        return false;
    }
    if s.len() == 1 {
        return true; // "."
    }
    let second = s.as_bytes()[1];
    if second == b'/' || second == b'\\' {
        return true; // "./" or ".\"
    }
    if second != b'.' {
        return false;
    }
    // ".." or "../" or "..\"
    s.len() == 2 || s.as_bytes()[2] == b'/' || s.as_bytes()[2] == b'\\'
}

/// Parse a pnpm-compatible package selector string.
///
/// Supported syntax:
/// - `<pattern>` — name match with glob (`*` wildcard)
/// - `<pattern>...` — package + all its dependencies
/// - `<pattern>^...` — only dependencies, exclude the package itself
/// - `...<pattern>` — package + all packages that depend on it
/// - `...^<pattern>` — only dependents, exclude the package itself
/// - `...<pattern>...` — both dependencies and dependents
/// - `!<pattern>` — exclusion
/// - `{./path}` — braced path selector
/// - `<pattern>{./path}` — combined name pattern + path selector
pub(super) fn parse_package_selector(raw: &str) -> PackageSelector {
    let mut s = raw;
    let mut exclude = false;
    let mut exclude_self = false;
    let mut include_dependencies = false;
    let mut include_dependents = false;

    // Check for ! prefix (exclude)
    if let Some(rest) = s.strip_prefix('!') {
        exclude = true;
        s = rest;
    }

    // Check for ... suffix (include dependencies)
    if let Some(rest) = s.strip_suffix("...") {
        include_dependencies = true;
        s = rest;
        if let Some(rest) = s.strip_suffix('^') {
            exclude_self = true;
            s = rest;
        }
    }

    // Check for ... prefix (include dependents)
    if let Some(rest) = s.strip_prefix("...") {
        include_dependents = true;
        s = rest;
        if let Some(rest) = s.strip_prefix('^') {
            exclude_self = true;
            s = rest;
        }
    }

    // Parse remaining: could be "namePattern{parentDir}", "{parentDir}", "namePattern", or path
    let (name_pattern, parent_dir) = if let Some(brace_start) = s.find('{') {
        if let Some(brace_end) = s.find('}') {
            let path_part = &s[brace_start + 1..brace_end];
            let name_part = if brace_start > 0 { Some(s[..brace_start].to_string()) } else { None };
            (name_part, Some(path_part.to_string()))
        } else {
            // Malformed — treat as name pattern
            (Some(s.to_string()), None)
        }
    } else if is_path_selector(s) {
        // Unbraced path selectors don't support traversal modifiers (matching pnpm).
        // Use braced syntax {./path}... for path + dependency traversal.
        include_dependencies = false;
        include_dependents = false;
        exclude_self = false;
        (None, Some(s.to_string()))
    } else if s.is_empty() {
        (None, None)
    } else {
        (Some(s.to_string()), None)
    };

    PackageSelector {
        name_pattern,
        parent_dir,
        include_dependencies,
        include_dependents,
        exclude_self,
        exclude,
    }
}

/// Filter packages from a dependency graph using pnpm-compatible selectors.
///
/// Returns node indices sorted alphabetically by package name.
pub(super) fn filter_packages(
    graph: &petgraph::graph::DiGraph<
        vite_workspace::PackageInfo,
        vite_workspace::DependencyType,
        vite_workspace::PackageIx,
    >,
    selectors: &[PackageSelector],
    cwd: &AbsolutePath,
) -> Vec<vite_workspace::PackageNodeIndex> {
    let mut included = HashSet::new();
    let mut excluded = HashSet::new();

    // If every selector is an exclusion, seed `included` with all non-root
    // packages so that exclusions subtract from the full set.
    let all_exclude = selectors.iter().all(|s| s.exclude);
    if all_exclude {
        for idx in graph.node_indices() {
            if !graph[idx].path.as_str().is_empty() {
                included.insert(idx);
            }
        }
    }

    for selector in selectors {
        let mut matched = HashSet::new();

        // Match by path (parent_dir) first
        if let Some(dir) = &selector.parent_dir {
            let filter_path = cwd.join(dir);
            if let Ok(canonical) = std::fs::canonicalize(filter_path.as_path()) {
                for idx in graph.node_indices() {
                    let pkg_abs = graph[idx].absolute_path.as_path();
                    if let Ok(pkg_canonical) = std::fs::canonicalize(pkg_abs) {
                        // Match if filter path equals or is a parent of pkg path
                        if pkg_canonical.starts_with(&canonical) {
                            matched.insert(idx);
                        }
                    }
                }
            }
        }

        // Match by name pattern (intersect with path results if both present)
        if let Some(pattern) = &selector.name_pattern {
            if let Ok(pat) = glob::Pattern::new(pattern) {
                if selector.parent_dir.is_some() {
                    // Both path and name: filter path matches by name
                    matched.retain(|&idx| {
                        let name = graph[idx].package_json.name.as_str();
                        pat.matches(name)
                    });
                } else {
                    // Name only: match all packages
                    for idx in graph.node_indices() {
                        let name = graph[idx].package_json.name.as_str();
                        if pat.matches(name) {
                            matched.insert(idx);
                        }
                    }
                }
            }
        }

        // Expand with dependency/dependent traversal
        let mut expanded = HashSet::new();

        for &node in &matched {
            if !selector.exclude_self {
                expanded.insert(node);
            }

            // Include transitive dependencies (follow outgoing edges: A→B means A depends on B)
            if selector.include_dependencies {
                let mut queue = VecDeque::new();
                let mut visited = HashSet::new();
                visited.insert(node);
                for neighbor in graph.neighbors_directed(node, Direction::Outgoing) {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                        expanded.insert(neighbor);
                    }
                }
                while let Some(current) = queue.pop_front() {
                    for neighbor in graph.neighbors_directed(current, Direction::Outgoing) {
                        if visited.insert(neighbor) {
                            queue.push_back(neighbor);
                            expanded.insert(neighbor);
                        }
                    }
                }
            }

            // Include transitive dependents (follow incoming edges)
            if selector.include_dependents {
                let mut queue = VecDeque::new();
                let mut visited = HashSet::new();
                visited.insert(node);
                for neighbor in graph.neighbors_directed(node, Direction::Incoming) {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                        expanded.insert(neighbor);
                    }
                }
                while let Some(current) = queue.pop_front() {
                    for neighbor in graph.neighbors_directed(current, Direction::Incoming) {
                        if visited.insert(neighbor) {
                            queue.push_back(neighbor);
                            expanded.insert(neighbor);
                        }
                    }
                }
            }
        }

        // If both flags set, also walk dependencies of dependents (matching pnpm behavior)
        if selector.include_dependencies && selector.include_dependents {
            let dependents: Vec<_> = expanded.iter().copied().collect();
            for dep_node in dependents {
                let mut queue = VecDeque::new();
                let mut visited = HashSet::new();
                visited.insert(dep_node);
                for neighbor in graph.neighbors_directed(dep_node, Direction::Outgoing) {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                        expanded.insert(neighbor);
                    }
                }
                while let Some(current) = queue.pop_front() {
                    for neighbor in graph.neighbors_directed(current, Direction::Outgoing) {
                        if visited.insert(neighbor) {
                            queue.push_back(neighbor);
                            expanded.insert(neighbor);
                        }
                    }
                }
            }
        }

        if selector.exclude {
            excluded.extend(expanded);
        } else {
            included.extend(expanded);
        }
    }

    // Return included minus excluded, in topological order
    let result: Vec<_> = included.difference(&excluded).copied().collect();
    topological_sort_packages(graph, &result)
}

/// Sort package indices in topological order (dependencies before dependents)
/// using Kahn's algorithm, with alphabetical tie-breaking for determinism.
///
/// Packages involved in dependency cycles are appended at the end in
/// alphabetical order, ensuring the command completes rather than failing.
pub(super) fn topological_sort_packages(
    graph: &petgraph::graph::DiGraph<
        vite_workspace::PackageInfo,
        vite_workspace::DependencyType,
        vite_workspace::PackageIx,
    >,
    selected: &[vite_workspace::PackageNodeIndex],
) -> Vec<vite_workspace::PackageNodeIndex> {
    let selected_set: FxHashSet<_> = selected.iter().copied().collect();

    // Count how many selected dependencies each selected package has
    // (Outgoing edges = dependencies in this graph)
    let mut dep_count: FxHashMap<vite_workspace::PackageNodeIndex, usize> = FxHashMap::default();
    for &idx in selected {
        let count = graph
            .neighbors_directed(idx, Direction::Outgoing)
            .filter(|n| selected_set.contains(n))
            .count();
        dep_count.insert(idx, count);
    }

    // BTreeMap keyed by name for deterministic alphabetical ordering among peers
    let mut ready: BTreeMap<&str, vite_workspace::PackageNodeIndex> = BTreeMap::new();
    for (&idx, &count) in &dep_count {
        if count == 0 {
            ready.insert(graph[idx].package_json.name.as_str(), idx);
        }
    }

    let mut result = Vec::with_capacity(selected.len());
    while let Some((_, idx)) = ready.pop_first() {
        result.push(idx);
        // Decrement dep counts for dependents (incoming edges = dependents)
        for dependent in graph.neighbors_directed(idx, Direction::Incoming) {
            if let Some(count) = dep_count.get_mut(&dependent) {
                *count -= 1;
                if *count == 0 {
                    ready.insert(graph[dependent].package_json.name.as_str(), dependent);
                }
            }
        }
    }

    // Cycle fallback: iteratively break cycles by forcing the alphabetically-first
    // remaining node, then continue Kahn's algorithm to correctly order any
    // non-cyclic dependents that become unblocked.
    let mut placed: FxHashSet<_> = result.iter().copied().collect();
    while result.len() < selected.len() {
        let mut remaining: Vec<_> =
            selected.iter().copied().filter(|idx| !placed.contains(idx)).collect();
        remaining.sort_by(|a, b| graph[*a].package_json.name.cmp(&graph[*b].package_json.name));

        let cyclic_names: Vec<&str> =
            remaining.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        tracing::debug!(
            "Circular dependencies detected among packages: {}. Breaking cycle at '{}'.",
            cyclic_names.join(", "),
            graph[remaining[0]].package_json.name
        );

        // Force-add the alphabetically-first remaining node to break the cycle
        let forced = remaining[0];
        result.push(forced);
        placed.insert(forced);

        // Decrement dep counts for its dependents, potentially freeing non-cyclic nodes
        for dependent in graph.neighbors_directed(forced, Direction::Incoming) {
            if let Some(count) = dep_count.get_mut(&dependent) {
                if *count > 0 {
                    *count -= 1;
                    if *count == 0 && !placed.contains(&dependent) {
                        ready.insert(graph[dependent].package_json.name.as_str(), dependent);
                    }
                }
            }
        }

        // Continue Kahn's algorithm with any newly freed nodes
        while let Some((_, idx)) = ready.pop_first() {
            result.push(idx);
            placed.insert(idx);
            for dependent in graph.neighbors_directed(idx, Direction::Incoming) {
                if let Some(count) = dep_count.get_mut(&dependent) {
                    if *count > 0 {
                        *count -= 1;
                        if *count == 0 && !placed.contains(&dependent) {
                            ready.insert(graph[dependent].package_json.name.as_str(), dependent);
                        }
                    }
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use vite_path::{AbsolutePathBuf, RelativePathBuf};
    use vite_workspace::{DependencyType, PackageInfo, PackageJson};

    use super::*;

    /// Build a test dependency graph:
    /// - app-a depends on lib-c
    /// - app-b has no workspace dependencies
    /// - lib-c has no workspace dependencies
    /// - root (workspace root, empty path)
    fn build_test_graph() -> petgraph::graph::DiGraph<
        vite_workspace::PackageInfo,
        vite_workspace::DependencyType,
        vite_workspace::PackageIx,
    > {
        let mut graph = petgraph::graph::DiGraph::default();

        let root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap().into(),
        });
        let app_a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "app-a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/app-a").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/app-a"))
                .unwrap()
                .into(),
        });
        let app_b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "app-b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/app-b").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/app-b"))
                .unwrap()
                .into(),
        });
        let lib_c = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "lib-c".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/lib-c").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/lib-c"))
                .unwrap()
                .into(),
        });

        // app-a depends on lib-c
        graph.add_edge(app_a, lib_c, DependencyType::Normal);

        let _ = (root, app_b); // suppress unused warnings
        graph
    }

    #[test]
    fn test_parse_package_selector_simple_name() {
        let sel = parse_package_selector("my-app");
        assert_eq!(sel.name_pattern.as_deref(), Some("my-app"));
        assert!(!sel.include_dependencies);
        assert!(!sel.include_dependents);
        assert!(!sel.exclude_self);
        assert!(!sel.exclude);
    }

    #[test]
    fn test_parse_package_selector_glob_pattern() {
        let sel = parse_package_selector("app-*");
        assert_eq!(sel.name_pattern.as_deref(), Some("app-*"));
        assert!(!sel.include_dependencies);
        assert!(!sel.include_dependents);
    }

    #[test]
    fn test_parse_package_selector_scoped_glob() {
        let sel = parse_package_selector("@myorg/*");
        assert_eq!(sel.name_pattern.as_deref(), Some("@myorg/*"));
    }

    #[test]
    fn test_parse_package_selector_with_dependencies() {
        let sel = parse_package_selector("app-a...");
        assert_eq!(sel.name_pattern.as_deref(), Some("app-a"));
        assert!(sel.include_dependencies);
        assert!(!sel.include_dependents);
        assert!(!sel.exclude_self);
    }

    #[test]
    fn test_parse_package_selector_with_dependencies_exclude_self() {
        let sel = parse_package_selector("app-a^...");
        assert_eq!(sel.name_pattern.as_deref(), Some("app-a"));
        assert!(sel.include_dependencies);
        assert!(!sel.include_dependents);
        assert!(sel.exclude_self);
    }

    #[test]
    fn test_parse_package_selector_with_dependents() {
        let sel = parse_package_selector("...lib-c");
        assert_eq!(sel.name_pattern.as_deref(), Some("lib-c"));
        assert!(!sel.include_dependencies);
        assert!(sel.include_dependents);
        assert!(!sel.exclude_self);
    }

    #[test]
    fn test_parse_package_selector_with_dependents_exclude_self() {
        let sel = parse_package_selector("...^lib-c");
        assert_eq!(sel.name_pattern.as_deref(), Some("lib-c"));
        assert!(!sel.include_dependencies);
        assert!(sel.include_dependents);
        assert!(sel.exclude_self);
    }

    #[test]
    fn test_parse_package_selector_exclude() {
        let sel = parse_package_selector("!app-b");
        assert_eq!(sel.name_pattern.as_deref(), Some("app-b"));
        assert!(sel.exclude);
    }

    #[test]
    fn test_parse_package_selector_exclude_with_dependencies() {
        let sel = parse_package_selector("!app-a...");
        assert_eq!(sel.name_pattern.as_deref(), Some("app-a"));
        assert!(sel.exclude);
        assert!(sel.include_dependencies);
    }

    #[test]
    fn test_filter_packages_simple_name() {
        let graph = build_test_graph();
        let selectors = vec![parse_package_selector("app-a")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        assert_eq!(names, vec!["app-a"]);
    }

    #[test]
    fn test_filter_packages_glob() {
        let graph = build_test_graph();
        let selectors = vec![parse_package_selector("app-*")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        assert_eq!(names, vec!["app-a", "app-b"]);
    }

    #[test]
    fn test_filter_packages_with_dependencies() {
        let graph = build_test_graph();
        let selectors = vec![parse_package_selector("app-a...")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // app-a depends on lib-c; topological order: lib-c first (dependency), then app-a
        assert_eq!(names, vec!["lib-c", "app-a"]);
    }

    #[test]
    fn test_filter_packages_dependencies_exclude_self() {
        let graph = build_test_graph();
        let selectors = vec![parse_package_selector("app-a^...")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // Only dependencies, not app-a itself
        assert_eq!(names, vec!["lib-c"]);
    }

    #[test]
    fn test_filter_packages_with_dependents() {
        let graph = build_test_graph();
        let selectors = vec![parse_package_selector("...lib-c")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // lib-c and all packages that depend on it (app-a); topological order: lib-c first
        assert_eq!(names, vec!["lib-c", "app-a"]);
    }

    #[test]
    fn test_filter_packages_exclude() {
        let graph = build_test_graph();
        let selectors = vec![parse_package_selector("app-*"), parse_package_selector("!app-b")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        assert_eq!(names, vec!["app-a"]);
    }

    #[test]
    fn test_is_path_selector() {
        // Valid path selectors
        assert!(is_path_selector("."));
        assert!(is_path_selector("./foo"));
        assert!(is_path_selector("./packages/app-a"));
        assert!(is_path_selector(".."));
        assert!(is_path_selector("../foo"));
        assert!(is_path_selector("../packages/app-a"));
        assert!(is_path_selector(".\\foo")); // Windows-style
        assert!(is_path_selector("..\\foo")); // Windows-style

        // Not path selectors
        assert!(!is_path_selector("foo"));
        assert!(!is_path_selector(".foo")); // dotfile name, not a path
        assert!(!is_path_selector("app-*"));
        assert!(!is_path_selector("@myorg/*"));
        assert!(!is_path_selector(""));
    }

    #[test]
    fn test_parse_package_selector_path() {
        let sel = parse_package_selector("./packages/app-a");
        assert_eq!(sel.name_pattern, None);
        assert_eq!(sel.parent_dir.as_deref(), Some("./packages/app-a"));
        assert!(!sel.include_dependencies);
        assert!(!sel.include_dependents);
        assert!(!sel.exclude_self);
        assert!(!sel.exclude);
    }

    #[test]
    fn test_parse_package_selector_path_ignores_traversal() {
        // Unbraced path selectors don't support traversal modifiers (matching pnpm).
        // The ... suffix is stripped but traversal flags are reset.
        let sel = parse_package_selector("./packages/app-a...");
        assert_eq!(sel.name_pattern, None);
        assert_eq!(sel.parent_dir.as_deref(), Some("./packages/app-a"));
        assert!(!sel.include_dependencies);
        assert!(!sel.include_dependents);
        assert!(!sel.exclude_self);
    }

    #[test]
    fn test_parse_package_selector_path_exclude() {
        let sel = parse_package_selector("!./packages/app-b");
        assert_eq!(sel.name_pattern, None);
        assert_eq!(sel.parent_dir.as_deref(), Some("./packages/app-b"));
        assert!(sel.exclude);
    }

    #[test]
    fn test_parse_package_selector_path_parent_dir() {
        let sel = parse_package_selector("../other-pkg");
        assert_eq!(sel.name_pattern, None);
        assert_eq!(sel.parent_dir.as_deref(), Some("../other-pkg"));
    }

    #[test]
    fn test_parse_package_selector_dot_only() {
        let sel = parse_package_selector(".");
        assert_eq!(sel.name_pattern, None);
        assert_eq!(sel.parent_dir.as_deref(), Some("."));
    }

    #[test]
    fn test_parse_package_selector_braced_path() {
        let sel = parse_package_selector("{./packages/app-a}");
        assert_eq!(sel.name_pattern, None);
        assert_eq!(sel.parent_dir.as_deref(), Some("./packages/app-a"));
        assert!(!sel.include_dependencies);
        assert!(!sel.include_dependents);
        assert!(!sel.exclude_self);
        assert!(!sel.exclude);
    }

    #[test]
    fn test_parse_package_selector_braced_path_deps() {
        let sel = parse_package_selector("{./packages/app-a}...");
        assert_eq!(sel.name_pattern, None);
        assert_eq!(sel.parent_dir.as_deref(), Some("./packages/app-a"));
        assert!(sel.include_dependencies);
        assert!(!sel.include_dependents);
    }

    #[test]
    fn test_parse_package_selector_braced_path_dependents() {
        let sel = parse_package_selector("...{./packages/app-a}");
        assert_eq!(sel.name_pattern, None);
        assert_eq!(sel.parent_dir.as_deref(), Some("./packages/app-a"));
        assert!(!sel.include_dependencies);
        assert!(sel.include_dependents);
    }

    #[test]
    fn test_parse_package_selector_name_and_path() {
        let sel = parse_package_selector("app-*{./packages}");
        assert_eq!(sel.name_pattern.as_deref(), Some("app-*"));
        assert_eq!(sel.parent_dir.as_deref(), Some("./packages"));
        assert!(!sel.include_dependencies);
        assert!(!sel.include_dependents);
    }

    #[test]
    fn test_parse_package_selector_both_directions() {
        let sel = parse_package_selector("...foo...");
        assert_eq!(sel.name_pattern.as_deref(), Some("foo"));
        assert!(sel.include_dependencies);
        assert!(sel.include_dependents);
        assert!(!sel.exclude_self);
    }

    #[test]
    fn test_parse_package_selector_braced_path_exclude() {
        let sel = parse_package_selector("!{./packages/app-b}");
        assert_eq!(sel.name_pattern, None);
        assert_eq!(sel.parent_dir.as_deref(), Some("./packages/app-b"));
        assert!(sel.exclude);
    }

    #[test]
    fn test_parse_package_selector_name_and_path_with_deps() {
        let sel = parse_package_selector("app-*{./packages}...");
        assert_eq!(sel.name_pattern.as_deref(), Some("app-*"));
        assert_eq!(sel.parent_dir.as_deref(), Some("./packages"));
        assert!(sel.include_dependencies);
    }

    #[test]
    fn test_parse_package_selector_malformed_brace() {
        // Missing closing brace — treated as name pattern
        let sel = parse_package_selector("{./packages/app-a");
        assert_eq!(sel.name_pattern.as_deref(), Some("{./packages/app-a"));
        assert_eq!(sel.parent_dir, None);
    }

    #[test]
    fn test_topological_sort_simple() {
        let graph = build_test_graph();
        // All non-root packages
        let all: Vec<_> =
            graph.node_indices().filter(|&idx| !graph[idx].path.as_str().is_empty()).collect();
        let sorted = super::topological_sort_packages(&graph, &all);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // app-b and lib-c have no deps, sorted alphabetically first
        // app-a depends on lib-c, so it comes after lib-c
        assert_eq!(names, vec!["app-b", "lib-c", "app-a"]);
    }

    #[test]
    fn test_topological_sort_with_cycles() {
        let mut graph = petgraph::graph::DiGraph::default();

        let root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap().into(),
        });
        let pkg_a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-a").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/pkg-a"))
                .unwrap()
                .into(),
        });
        let pkg_b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-b").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/pkg-b"))
                .unwrap()
                .into(),
        });
        let pkg_c = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "pkg-c".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/pkg-c").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/pkg-c"))
                .unwrap()
                .into(),
        });

        // Circular: pkg-a <-> pkg-b
        graph.add_edge(pkg_a, pkg_b, DependencyType::Normal);
        graph.add_edge(pkg_b, pkg_a, DependencyType::Normal);
        // pkg-c has no dependencies
        let _ = root;

        let selected = vec![pkg_a, pkg_b, pkg_c];
        let sorted = super::topological_sort_packages(&graph, &selected);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // pkg-c has no deps, comes first; pkg-a and pkg-b are in a cycle, appended alphabetically
        assert_eq!(names, vec!["pkg-c", "pkg-a", "pkg-b"]);
    }

    #[test]
    fn test_topological_sort_cycle_with_dependent() {
        let mut graph = petgraph::graph::DiGraph::default();

        let _root = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "root".into(), ..Default::default() },
            path: RelativePathBuf::default(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap().into(),
        });
        let a = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "a".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/a").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/a"))
                .unwrap()
                .into(),
        });
        let b = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "b".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/b").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/b"))
                .unwrap()
                .into(),
        });
        let aa = graph.add_node(PackageInfo {
            package_json: PackageJson { name: "aa".into(), ..Default::default() },
            path: RelativePathBuf::try_from("packages/aa").unwrap(),
            absolute_path: AbsolutePathBuf::new(PathBuf::from("/workspace/packages/aa"))
                .unwrap()
                .into(),
        });

        // Cycle: a <-> b
        graph.add_edge(a, b, DependencyType::Normal);
        graph.add_edge(b, a, DependencyType::Normal);
        // aa depends on b (non-cyclic dependent)
        graph.add_edge(aa, b, DependencyType::Normal);

        let selected = vec![a, b, aa];
        let sorted = super::topological_sort_packages(&graph, &selected);
        let names: Vec<&str> =
            sorted.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // Force 'a' first (alphabetical cycle break), frees 'b', then 'aa' follows
        assert_eq!(names, vec!["a", "b", "aa"]);
    }

    #[test]
    fn test_filter_packages_multiple_inclusion() {
        let graph = build_test_graph();
        let selectors = vec![parse_package_selector("app-a"), parse_package_selector("lib-c")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // Union of both selectors, topological order: lib-c first (app-a depends on it)
        assert_eq!(names, vec!["lib-c", "app-a"]);
    }

    #[test]
    fn test_filter_packages_exclusion_only() {
        let graph = build_test_graph();
        let selectors = vec![parse_package_selector("!app-b")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // All non-root packages minus app-b, in topological order
        assert_eq!(names, vec!["lib-c", "app-a"]);
    }

    #[test]
    fn test_filter_packages_exclusion_only_multiple() {
        let graph = build_test_graph();
        let selectors = vec![parse_package_selector("!app-a"), parse_package_selector("!app-b")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        assert_eq!(names, vec!["lib-c"]);
    }

    #[test]
    fn test_filter_packages_exclusion_only_all_excluded() {
        let graph = build_test_graph();
        let selectors = vec![
            parse_package_selector("!app-a"),
            parse_package_selector("!app-b"),
            parse_package_selector("!lib-c"),
        ];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_packages_exclusion_nonexistent() {
        let graph = build_test_graph();
        // Excluding a package that doesn't exist should return all non-root packages
        let selectors = vec![parse_package_selector("!no-such-pkg")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        assert_eq!(names, vec!["app-b", "lib-c", "app-a"]);
    }

    #[test]
    fn test_filter_packages_mixed_inclusion_matches_nothing() {
        let graph = build_test_graph();
        // An inclusion selector that matches nothing + an exclusion selector
        // should return empty (the inclusion intent found nothing)
        let selectors = vec![parse_package_selector("app-x"), parse_package_selector("!app-b")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_packages_glob_star_includes_root() {
        let graph = build_test_graph();
        // `*` matches all package names including root
        let selectors = vec![parse_package_selector("*")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        let names: Vec<&str> =
            result.iter().map(|&idx| graph[idx].package_json.name.as_str()).collect();
        // All 4 nodes including root, in topological order with alphabetical tie-breaking
        assert_eq!(names, vec!["app-b", "lib-c", "app-a", "root"]);
    }

    #[test]
    fn test_filter_packages_exclusion_star_returns_empty() {
        let graph = build_test_graph();
        // `!*` excludes everything, returns empty
        let selectors = vec![parse_package_selector("!*")];
        let dummy_cwd = AbsolutePathBuf::new(PathBuf::from("/workspace")).unwrap();
        let result = filter_packages(&graph, &selectors, &dummy_cwd);
        assert!(result.is_empty());
    }
}
