use std::{collections::VecDeque, fs};

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vt_path::AbsolutePath;
use vt_str::Str;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    pub id: Str,
    pub name: Str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<Str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<Str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_at: Option<Str>,
    pub kind: NodeKind,
    pub delivery: Vec<Delivery>,
    #[serde(default)]
    pub aliases: Vec<Str>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NodeKind {
    Package,
    Tool,
    Engine,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Delivery {
    Dependency,
    Bundled,
    Compiled,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    pub from: Str,
    pub to: Str,
    pub relationship: Relationship,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Relationship {
    DependsOn,
    Bundles,
    Uses,
    Compiles,
}

impl Relationship {
    fn display(self) -> &'static str {
        match self {
            Self::DependsOn => "depends on",
            Self::Bundles => "bundles",
            Self::Uses => "uses",
            Self::Compiles => "compiles",
        }
    }

    fn is_downstream_engine_relationship(self) -> bool {
        matches!(self, Self::Uses | Self::Compiles)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Local,
    Global,
}

impl Scope {
    pub const fn display(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Global => "global",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub scope: Scope,
    pub path: Str,
    pub vite_plus_version: Str,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub schema_version: u32,
    pub source: Source,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Error)]
pub enum ToolchainError {
    #[error("failed to read toolchain manifest at {path}: {source}")]
    Read { path: Box<AbsolutePath>, source: std::io::Error },
    #[error("failed to parse toolchain manifest at {path}: {source}")]
    Parse { path: Box<AbsolutePath>, source: serde_json::Error },
    #[error("unsupported toolchain manifest schema version {0}")]
    UnsupportedSchema(u32),
    #[error("invalid toolchain manifest: {0}")]
    InvalidManifest(Str),
    #[error("`{0}` is not in the Vite+ toolchain")]
    UnknownFilter(Str),
    #[error("failed to create toolchain JSON: {0}")]
    Serialize(#[from] serde_json::Error),
}

pub fn load_manifest(path: &AbsolutePath) -> Result<Manifest, ToolchainError> {
    let content = fs::read_to_string(path)
        .map_err(|source| ToolchainError::Read { path: path.into(), source })?;
    let manifest = serde_json::from_str(&content)
        .map_err(|source| ToolchainError::Parse { path: path.into(), source })?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

pub fn validate_manifest(manifest: &Manifest) -> Result<(), ToolchainError> {
    if manifest.schema_version != 1 {
        return Err(ToolchainError::UnsupportedSchema(manifest.schema_version));
    }

    let mut ids = FxHashSet::default();
    let mut node_ids_by_label = FxHashMap::default();
    for node in &manifest.nodes {
        if node.id.is_empty() || node.name.is_empty() {
            return Err(ToolchainError::InvalidManifest(
                "node IDs and names must not be empty".into(),
            ));
        }
        if node.version.as_ref().is_some_and(|version| version.is_empty())
            || node.revision.as_ref().is_some_and(|revision| revision.is_empty())
            || node.built_at.as_ref().is_some_and(|built_at| built_at.is_empty())
        {
            return Err(ToolchainError::InvalidManifest(vt_str::format!(
                "node `{}` has an empty version, revision, or build time",
                node.id
            )));
        }
        if node.built_at.as_ref().is_some_and(|built_at| !is_utc_build_time(built_at)) {
            return Err(ToolchainError::InvalidManifest(vt_str::format!(
                "node `{}` has an invalid UTC build time",
                node.id
            )));
        }
        if node.version.is_some() && node.built_at.is_some() {
            return Err(ToolchainError::InvalidManifest(vt_str::format!(
                "node `{}` must not have both a version and build time",
                node.id
            )));
        }
        if node.version.is_none() && node.revision.is_none() && node.built_at.is_none() {
            return Err(ToolchainError::InvalidManifest(vt_str::format!(
                "node `{}` must have a version, revision, or build time",
                node.id
            )));
        }
        if !ids.insert(node.id.as_str()) {
            return Err(ToolchainError::InvalidManifest(vt_str::format!(
                "duplicate node ID `{}`",
                node.id
            )));
        }
        for label in node_labels(node) {
            if label.is_empty() {
                return Err(ToolchainError::InvalidManifest(vt_str::format!(
                    "node `{}` has an empty name or alias",
                    node.id
                )));
            }
            if let Some(existing_node) = node_ids_by_label.insert(label, node.id.as_str())
                && existing_node != node.id.as_str()
            {
                return Err(ToolchainError::InvalidManifest(vt_str::format!(
                    "filter label `{label}` is shared by `{existing_node}` and `{}`",
                    node.id
                )));
            }
        }
    }

    for edge in &manifest.edges {
        if !ids.contains(edge.from.as_str()) || !ids.contains(edge.to.as_str()) {
            return Err(ToolchainError::InvalidManifest(vt_str::format!(
                "edge references an unknown node: {} -> {}",
                edge.from,
                edge.to
            )));
        }
    }

    Ok(())
}

fn is_utc_build_time(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
}

pub fn node_by_id<'a>(manifest: &'a Manifest, id: &str) -> Option<&'a Node> {
    manifest.nodes.iter().find(|node| node.id == id)
}

fn node_labels(node: &Node) -> impl Iterator<Item = &str> {
    [node.id.as_str(), node.name.as_str()].into_iter().chain(node.aliases.iter().map(Str::as_str))
}

fn node_by_label<'a>(manifest: &'a Manifest, label: &str) -> Option<&'a Node> {
    manifest.nodes.iter().find(|node| node_labels(node).any(|candidate| candidate == label))
}

pub fn root_version(manifest: &Manifest) -> Option<&str> {
    node_by_id(manifest, "vite-plus").and_then(|node| node.version.as_deref())
}

pub fn build_report<T: AsRef<str>>(
    manifest: &Manifest,
    filters: &[T],
    source: Source,
) -> Result<Report, ToolchainError> {
    validate_manifest(manifest)?;
    let filtered = filter_manifest(manifest, filters)?;
    Ok(Report {
        schema_version: filtered.schema_version,
        source,
        nodes: filtered.nodes,
        edges: filtered.edges,
    })
}

pub fn filter_manifest<T: AsRef<str>>(
    manifest: &Manifest,
    filters: &[T],
) -> Result<Manifest, ToolchainError> {
    if filters.is_empty() {
        return Ok(manifest.clone());
    }

    let mut node_indices = FxHashMap::default();
    let mut node_indices_by_label = FxHashMap::default();
    for (index, node) in manifest.nodes.iter().enumerate() {
        node_indices.insert(node.id.as_str(), index);
        for label in node_labels(node) {
            node_indices_by_label.insert(label, index);
        }
    }

    let mut keep_nodes = vec![false; manifest.nodes.len()];
    let mut keep_edges = vec![false; manifest.edges.len()];
    for filter in filters {
        let filter = filter.as_ref();
        let Some(&matched) = node_indices_by_label.get(filter) else {
            return Err(ToolchainError::UnknownFilter(filter.into()));
        };
        keep_nodes[matched] = true;

        let mut visited_ancestors = vec![false; manifest.nodes.len()];
        visited_ancestors[matched] = true;
        let mut ancestors = VecDeque::from([matched]);
        while let Some(node_index) = ancestors.pop_front() {
            let node_id = manifest.nodes[node_index].id.as_str();
            for (edge_index, edge) in manifest.edges.iter().enumerate() {
                if edge.to != node_id {
                    continue;
                }
                keep_edges[edge_index] = true;
                let parent = node_indices[edge.from.as_str()];
                keep_nodes[parent] = true;
                if !visited_ancestors[parent] {
                    visited_ancestors[parent] = true;
                    ancestors.push_back(parent);
                }
            }
        }

        let mut visited_downstream = vec![false; manifest.nodes.len()];
        visited_downstream[matched] = true;
        let mut downstream = VecDeque::from([matched]);
        while let Some(node_index) = downstream.pop_front() {
            let node_id = manifest.nodes[node_index].id.as_str();
            for (edge_index, edge) in manifest.edges.iter().enumerate() {
                if edge.from != node_id || !edge.relationship.is_downstream_engine_relationship() {
                    continue;
                }
                keep_edges[edge_index] = true;
                let child = node_indices[edge.to.as_str()];
                keep_nodes[child] = true;
                if !visited_downstream[child] {
                    visited_downstream[child] = true;
                    downstream.push_back(child);
                }
            }
        }
    }

    Ok(Manifest {
        schema_version: manifest.schema_version,
        nodes: manifest
            .nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| keep_nodes[*index])
            .map(|(_, node)| node.clone())
            .collect(),
        edges: manifest
            .edges
            .iter()
            .enumerate()
            .filter(|(index, _)| keep_edges[*index])
            .map(|(_, edge)| edge.clone())
            .collect(),
    })
}

pub fn render(report: &Report) -> Str {
    let mut output = vt_str::format!("Vite+ toolchain ({})\n\n", report.source.scope.display());
    let nodes =
        report.nodes.iter().map(|node| (node.id.as_str(), node)).collect::<FxHashMap<_, _>>();
    let mut children: FxHashMap<&str, Vec<&Edge>> = FxHashMap::default();
    let mut nodes_with_parents = FxHashSet::default();
    for edge in &report.edges {
        children.entry(edge.from.as_str()).or_default().push(edge);
        nodes_with_parents.insert(edge.to.as_str());
    }

    let roots = report
        .nodes
        .iter()
        .filter(|node| !nodes_with_parents.contains(node.id.as_str()))
        .collect::<Vec<_>>();
    for (root_index, root) in roots.iter().enumerate() {
        if root_index > 0 {
            output.push('\n');
        }
        output.push_str(&display_node(root));
        output.push('\n');
        let mut path = FxHashSet::from_iter([root.id.as_str()]);
        render_children(&mut output, root.id.as_str(), "", &nodes, &children, &mut path);
    }
    output
}

fn render_children<'a>(
    output: &mut Str,
    node_id: &'a str,
    prefix: &str,
    nodes: &FxHashMap<&'a str, &'a Node>,
    children: &FxHashMap<&'a str, Vec<&'a Edge>>,
    path: &mut FxHashSet<&'a str>,
) {
    let Some(edges) = children.get(node_id) else {
        return;
    };
    for (index, edge) in edges.iter().enumerate() {
        let is_last = index + 1 == edges.len();
        let connector = if is_last { "`-- " } else { "|-- " };
        let Some(node) = nodes.get(edge.to.as_str()) else {
            continue;
        };
        output.push_str(&vt_str::format!(
            "{prefix}{connector}{} {}",
            edge.relationship.display(),
            display_node(node)
        ));
        output.push('\n');

        if path.insert(node.id.as_str()) {
            let child_prefix = vt_str::format!("{prefix}{}", if is_last { "    " } else { "|   " });
            render_children(output, node.id.as_str(), child_prefix.as_str(), nodes, children, path);
            path.remove(node.id.as_str());
        }
    }
}

fn display_node(node: &Node) -> Str {
    match (&node.version, &node.built_at, &node.revision) {
        (_, Some(built_at), Some(revision)) => {
            vt_str::format!("{} (built {built_at}, revision {revision})", node.name)
        }
        (_, Some(built_at), None) => vt_str::format!("{} (built {built_at})", node.name),
        (Some(version), None, Some(revision)) => {
            vt_str::format!("{}@{} ({revision})", node.name, version)
        }
        (Some(version), None, None) => vt_str::format!("{}@{}", node.name, version),
        (None, None, Some(revision)) => vt_str::format!("{} (revision {revision})", node.name),
        (None, None, None) => node.name.clone(),
    }
}

pub fn render_json(report: &Report) -> Result<Str, ToolchainError> {
    let mut output: Str = serde_json::to_string_pretty(report)?.into();
    output.push('\n');
    Ok(output)
}

pub fn why_hint<T: AsRef<str>>(manifest: &Manifest, queries: &[T]) -> Option<Str> {
    let mut matched_ids = FxHashSet::default();
    let mut matched_nodes = Vec::new();
    for query in queries {
        let Some(node) = node_by_label(manifest, query.as_ref()) else {
            continue;
        };
        if matched_ids.insert(node.id.as_str()) {
            matched_nodes.push(node);
        }
    }
    if matched_nodes.is_empty() {
        return None;
    }

    let mut provided = Str::default();
    let mut filters = Str::default();
    for (index, node) in matched_nodes.iter().enumerate() {
        if index > 0 {
            provided.push_str(", ");
            filters.push(' ');
        }
        provided.push_str(&display_node(node));
        filters.push_str(node.name.as_str());
    }
    Some(vt_str::format!(
        "Vite+ also provides {provided} through its toolchain.\nRun `vp toolchain {filters}` to show these versions and relationships."
    ))
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn manifest() -> Manifest {
        serde_json::from_str(
            r#"{
              "schemaVersion": 1,
              "nodes": [
                {"id":"vite-plus","name":"vite-plus","version":"1.0.0","kind":"package","delivery":["dependency"],"aliases":[]},
                {"id":"core","name":"@scope/core","version":"1.0.0","kind":"package","delivery":["dependency"],"aliases":["core"]},
                {"id":"vite","name":"vite","version":"8.0.0","kind":"tool","delivery":["bundled"],"aliases":[]},
                {"id":"rolldown","name":"rolldown","version":"1.0.0","kind":"tool","delivery":["bundled","compiled"],"aliases":[]},
                {"id":"oxc","name":"oxc","version":"0.1.0","kind":"engine","delivery":["compiled"],"aliases":[]},
                {"id":"oxc-resolver","name":"oxc-resolver","version":"1.0.0","kind":"engine","delivery":["compiled"],"aliases":[]},
                {"id":"vitest","name":"vitest","version":"4.0.0","kind":"tool","delivery":["dependency"],"aliases":[]}
              ],
              "edges": [
                {"from":"vite-plus","to":"core","relationship":"depends-on"},
                {"from":"core","to":"vite","relationship":"bundles"},
                {"from":"vite","to":"rolldown","relationship":"uses"},
                {"from":"core","to":"rolldown","relationship":"bundles"},
                {"from":"rolldown","to":"oxc","relationship":"compiles"},
                {"from":"rolldown","to":"oxc-resolver","relationship":"compiles"},
                {"from":"vite-plus","to":"vitest","relationship":"depends-on"}
              ]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn vite_filter_keeps_ownership_and_engine_chain_without_unrelated_edges() {
        let filtered = filter_manifest(&manifest(), &["vite"]).unwrap();
        assert_eq!(
            filtered.nodes.iter().map(|node| node.id.as_str()).collect::<Vec<_>>(),
            ["vite-plus", "core", "vite", "rolldown", "oxc", "oxc-resolver"]
        );
        assert_eq!(
            filtered
                .edges
                .iter()
                .map(|edge| (edge.from.as_str(), edge.to.as_str()))
                .collect::<Vec<_>>(),
            [
                ("vite-plus", "core"),
                ("core", "vite"),
                ("vite", "rolldown"),
                ("rolldown", "oxc"),
                ("rolldown", "oxc-resolver")
            ]
        );
    }

    #[test]
    fn multiple_filters_form_a_stable_union() {
        let filtered = filter_manifest(&manifest(), &["vite", "vitest"]).unwrap();
        assert_eq!(
            filtered.nodes.iter().map(|node| node.id.as_str()).collect::<Vec<_>>(),
            ["vite-plus", "core", "vite", "rolldown", "oxc", "oxc-resolver", "vitest"]
        );
    }

    #[test]
    fn overlapping_filters_are_order_independent() {
        let oxc_then_vite = filter_manifest(&manifest(), &["oxc", "vite"]).unwrap();
        let vite_then_oxc = filter_manifest(&manifest(), &["vite", "oxc"]).unwrap();

        assert_eq!(oxc_then_vite, vite_then_oxc);
        assert!(oxc_then_vite.edges.iter().any(|edge| {
            edge.from == "rolldown"
                && edge.to == "oxc-resolver"
                && edge.relationship == Relationship::Compiles
        }));
    }

    #[test]
    fn aliases_match() {
        let filtered = filter_manifest(&manifest(), &["core"]).unwrap();
        assert!(filtered.nodes.iter().any(|node| node.id == "core"));
    }

    #[test]
    fn validation_allows_a_nodes_alias_to_repeat_its_id() {
        let mut manifest = manifest();
        manifest.nodes[1].aliases.push("core".into());
        validate_manifest(&manifest).unwrap();
    }

    #[test]
    fn validation_rejects_filter_labels_shared_by_different_nodes() {
        let mut manifest = manifest();
        manifest.nodes[6].aliases.push("vite".into());
        let error = validate_manifest(&manifest).unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid toolchain manifest: filter label `vite` is shared by `vite` and `vitest`"
        );
    }

    #[test]
    fn unknown_filter_is_an_error() {
        let error = filter_manifest(&manifest(), &["rollup"]).unwrap_err();
        assert_eq!(error.to_string(), "`rollup` is not in the Vite+ toolchain");
    }

    #[test]
    fn human_tree_repeats_shared_nodes_in_the_full_graph() {
        let report = build_report(
            &manifest(),
            &[] as &[&str],
            Source {
                scope: Scope::Local,
                path: "/project/node_modules/vite-plus".into(),
                vite_plus_version: "1.0.0".into(),
            },
        )
        .unwrap();
        assert_eq!(
            render(&report),
            "Vite+ toolchain (local)\n\
             \n\
             vite-plus@1.0.0\n\
             |-- depends on @scope/core@1.0.0\n\
             |   |-- bundles vite@8.0.0\n\
             |   |   `-- uses rolldown@1.0.0\n\
             |   |       |-- compiles oxc@0.1.0\n\
             |   |       `-- compiles oxc-resolver@1.0.0\n\
             |   `-- bundles rolldown@1.0.0\n\
             |       |-- compiles oxc@0.1.0\n\
             |       `-- compiles oxc-resolver@1.0.0\n\
             `-- depends on vitest@4.0.0\n"
        );
    }

    #[test]
    fn why_hint_uses_all_matching_queries() {
        assert_eq!(
            why_hint(&manifest(), &["vite", "vitest"]).as_deref(),
            Some(
                "Vite+ also provides vite@8.0.0, vitest@4.0.0 through its toolchain.\n\
                 Run `vp toolchain vite vitest` to show these versions and relationships."
            )
        );
    }

    #[test]
    fn why_hint_keeps_matching_queries() {
        assert_eq!(
            why_hint(&manifest(), &["vite", "react"]).as_deref(),
            Some(
                "Vite+ also provides vite@8.0.0 through its toolchain.\n\
                 Run `vp toolchain vite` to show these versions and relationships."
            )
        );
    }

    #[test]
    fn build_time_replaces_a_placeholder_version_in_human_output() {
        let node = Node {
            id: "vite-task".into(),
            name: "vite-task".into(),
            version: None,
            revision: Some("ebe583739b0b1e7828199b9ee9dd52273fa2fd20".into()),
            built_at: Some("2026-08-06T09:30:00Z".into()),
            kind: NodeKind::Tool,
            delivery: vec![Delivery::Compiled],
            aliases: Vec::new(),
        };
        assert_eq!(
            display_node(&node),
            "vite-task (built 2026-08-06T09:30:00Z, revision ebe583739b0b1e7828199b9ee9dd52273fa2fd20)"
        );
    }

    #[test]
    fn manifest_accepts_a_revision_before_the_native_build_exists() {
        let mut manifest = manifest();
        let node = node_by_id(&manifest, "vite").unwrap().clone();
        manifest.nodes.push(Node {
            id: "vite-task".into(),
            name: "vite-task".into(),
            version: None,
            revision: Some("ebe583739b0b1e7828199b9ee9dd52273fa2fd20".into()),
            built_at: None,
            kind: NodeKind::Tool,
            delivery: vec![Delivery::Compiled],
            aliases: Vec::new(),
        });
        manifest.edges.push(Edge {
            from: node.id,
            to: "vite-task".into(),
            relationship: Relationship::Compiles,
        });
        validate_manifest(&manifest).unwrap();
    }

    #[test]
    fn manifest_rejects_invalid_or_ambiguous_build_times() {
        let mut manifest = manifest();
        manifest.nodes[0].built_at = Some("2026-08-06 09:30:00Z".into());
        assert!(matches!(
            validate_manifest(&manifest),
            Err(ToolchainError::InvalidManifest(message))
                if message.contains("invalid UTC build time")
        ));

        manifest.nodes[0].built_at = Some("2026-08-06T09:30:00Z".into());
        assert!(matches!(
            validate_manifest(&manifest),
            Err(ToolchainError::InvalidManifest(message))
                if message.contains("both a version and build time")
        ));
    }

    #[test]
    fn manifest_round_trips_from_disk() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("toolchain.json");
        fs::write(&path, serde_json::to_string(&manifest()).unwrap()).unwrap();
        let path = vt_path::AbsolutePathBuf::new(path).unwrap();
        assert_eq!(load_manifest(&path).unwrap(), manifest());
    }
}
