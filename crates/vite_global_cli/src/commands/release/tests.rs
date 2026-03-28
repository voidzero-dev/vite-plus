//! Unit tests for the split release modules.
//!
//! The release flow spans planning, security validation, git tag handling, manifest rewriting,
//! and user-facing summaries. These tests intentionally stay close to the module boundary rather
//! than trying to boot a full end-to-end workspace, so they can validate the core rules quickly
//! even when external workspace fixtures are unavailable.

use std::path::PathBuf;

use vite_path::{AbsolutePathBuf, RelativePathBuf};
use vite_str::Str;
use vite_workspace::{PackageInfo, PackageJson, PackageNodeIndex};

use super::*;

fn test_absolute_path(suffix: &str) -> AbsolutePathBuf {
    #[cfg(windows)]
    let base = PathBuf::from(format!("C:\\workspace{}", suffix.replace('/', "\\")));
    #[cfg(not(windows))]
    let base = PathBuf::from(format!("/workspace{suffix}"));
    AbsolutePathBuf::new(base).unwrap()
}

fn build_test_package_graph()
-> petgraph::graph::DiGraph<PackageInfo, DependencyType, vite_workspace::PackageIx> {
    let mut graph = petgraph::graph::DiGraph::default();

    let _root = graph.add_node(PackageInfo {
        package_json: PackageJson { name: "root".into(), ..Default::default() },
        path: RelativePathBuf::default(),
        absolute_path: test_absolute_path("").into(),
    });
    let pkg_a = graph.add_node(PackageInfo {
        package_json: PackageJson { name: "pkg-a".into(), ..Default::default() },
        path: RelativePathBuf::try_from("packages/pkg-a").unwrap(),
        absolute_path: test_absolute_path("/packages/pkg-a").into(),
    });
    let pkg_b = graph.add_node(PackageInfo {
        package_json: PackageJson { name: "pkg-b".into(), ..Default::default() },
        path: RelativePathBuf::try_from("packages/pkg-b").unwrap(),
        absolute_path: test_absolute_path("/packages/pkg-b").into(),
    });
    let pkg_c = graph.add_node(PackageInfo {
        package_json: PackageJson { name: "pkg-c".into(), ..Default::default() },
        path: RelativePathBuf::try_from("packages/pkg-c").unwrap(),
        absolute_path: test_absolute_path("/packages/pkg-c").into(),
    });

    graph.add_edge(pkg_a, pkg_b, DependencyType::Normal);

    let _ = pkg_c;
    graph
}

fn test_package_manager(client: PackageManagerType, version: &str) -> PackageManager {
    PackageManager {
        client,
        package_name: client.to_string().into(),
        version: Str::from(version),
        hash: None,
        bin_name: client.to_string().into(),
        workspace_root: test_absolute_path(""),
        is_monorepo: false,
        install_dir: test_absolute_path("/install"),
    }
}

fn make_workspace_package(
    graph: &petgraph::graph::DiGraph<PackageInfo, DependencyType, vite_workspace::PackageIx>,
    node: PackageNodeIndex,
    selection_order: usize,
) -> WorkspacePackage {
    let package = &graph[node];
    WorkspacePackage {
        node,
        name: package.package_json.name.to_string(),
        known_names: vec![package.package_json.name.to_string()],
        retired_names: Vec::new(),
        release_paths: vec![package.path.as_str().to_owned()],
        selection_order,
        manifest_path: package.absolute_path.join("package.json"),
        package_path: package.absolute_path.to_absolute_path_buf(),
        manifest_contents: r#"{"name":"pkg","version":"1.0.0"}"#.into(),
        manifest: PackageManifest {
            name: package.package_json.name.to_string(),
            version: "1.0.0".into(),
            ..Default::default()
        },
    }
}

fn make_release_plan(name: &str, scripts: &[&str], check_scripts: &[&str]) -> PackageReleasePlan {
    PackageReleasePlan {
        name: name.to_string(),
        known_names: vec![name.to_string()],
        retired_names: Vec::new(),
        package_path: test_absolute_path(&format!("/packages/{name}")),
        manifest_path: test_absolute_path(&format!("/packages/{name}/package.json")),
        manifest_contents: format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
        manifest: PackageManifest {
            name: name.to_string(),
            version: "1.0.0".into(),
            ..Default::default()
        },
        current_version: Version::parse("1.0.0").unwrap(),
        next_version: Version::parse("1.0.1").unwrap(),
        level: VersionBump::Patch,
        commits: Vec::new(),
        changelog_path: test_absolute_path(&format!("/packages/{name}/CHANGELOG.md")),
        access: None,
        publish_tag: None,
        publish_provenance: None,
        repository_url: Some(String::from("https://github.com/voidzero-dev/vite-plus.git")),
        protocol_summary: DependencyProtocolSummary::default(),
        tag_name: format!("release/{name}/v1.0.1"),
        scripts: scripts.iter().map(|script| (*script).to_string()).collect(),
        check_scripts: check_scripts.iter().map(|script| (*script).to_string()).collect(),
    }
}

fn make_release_options() -> ReleaseOptions {
    ReleaseOptions {
        dry_run: false,
        skip_publish: false,
        first_release: false,
        changelog: false,
        version: None,
        preid: None,
        otp: None,
        projects: None,
        git_tag: true,
        git_commit: true,
        run_checks: true,
        yes: false,
    }
}

fn github_hosted_trusted_publish_context() -> TrustedPublishContext {
    TrustedPublishContext::from_env(|key| match key {
        "GITHUB_ACTIONS" => Some(String::from("true")),
        "RUNNER_ENVIRONMENT" => Some(String::from("github-hosted")),
        "GITHUB_REPOSITORY" => Some(String::from("voidzero-dev/vite-plus")),
        "GITHUB_WORKFLOW" => Some(String::from("Release")),
        "GITHUB_WORKFLOW_REF" => Some(String::from(
            "voidzero-dev/vite-plus/.github/workflows/release.yml@refs/heads/main",
        )),
        _ => None,
    })
}

fn gitlab_trusted_publish_context() -> TrustedPublishContext {
    TrustedPublishContext::from_env(|key| match key {
        "GITLAB_CI" => Some(String::from("true")),
        _ => None,
    })
}

fn circleci_trusted_publish_context() -> TrustedPublishContext {
    TrustedPublishContext::from_env(|key| match key {
        "CIRCLECI" => Some(String::from("true")),
        _ => None,
    })
}

#[test]
fn parse_github_repo_slug_supports_common_remote_formats() {
    assert_eq!(
        parse_github_repo_slug("git@github.com:voidzero-dev/vite-plus.git"),
        Some("voidzero-dev/vite-plus".into())
    );
    assert_eq!(
        parse_github_repo_slug("https://github.com/voidzero-dev/vite-plus.git"),
        Some("voidzero-dev/vite-plus".into())
    );
    assert_eq!(
        parse_github_repo_slug("github:voidzero-dev/vite-plus"),
        Some("voidzero-dev/vite-plus".into())
    );
    assert_eq!(parse_github_repo_slug("https://example.com/acme/repo.git"), None);
}

#[test]
fn repository_url_reads_string_and_object_forms() {
    let manifest: PackageManifest = serde_json::from_str(
        r#"{
            "name": "@scope/pkg-a",
            "version": "1.0.0",
            "repository": "https://github.com/voidzero-dev/vite-plus.git"
        }"#,
    )
    .unwrap();
    assert_eq!(manifest.repository_url(), Some("https://github.com/voidzero-dev/vite-plus.git"));

    let manifest: PackageManifest = serde_json::from_str(
        r#"{
            "name": "@scope/pkg-b",
            "version": "1.0.0",
            "repository": {
                "type": "git",
                "url": "git@github.com:voidzero-dev/vite-plus.git"
            }
        }"#,
    )
    .unwrap();
    assert_eq!(manifest.repository_url(), Some("git@github.com:voidzero-dev/vite-plus.git"));
}

#[test]
fn render_release_command_keeps_relevant_flags() {
    let command = render_release_command(
        &ReleaseOptions {
            dry_run: false,
            skip_publish: true,
            first_release: true,
            changelog: true,
            version: None,
            preid: Some("alpha".into()),
            otp: None,
            projects: Some(vec!["@scope/pkg-a".into(), "@scope/pkg-b".into()]),
            git_tag: false,
            git_commit: true,
            run_checks: true,
            yes: false,
        },
        true,
        true,
    );

    assert_eq!(
        command,
        "vp release --first-release --changelog --preid alpha --projects @scope/pkg-a,@scope/pkg-b --no-git-tag --skip-publish --dry-run"
    );
}

#[test]
fn render_release_command_uses_yes_for_non_interactive_runs() {
    let command = render_release_command(
        &ReleaseOptions {
            dry_run: false,
            skip_publish: false,
            first_release: false,
            changelog: false,
            version: None,
            preid: None,
            otp: None,
            projects: None,
            git_tag: true,
            git_commit: true,
            run_checks: true,
            yes: false,
        },
        false,
        true,
    );

    assert_eq!(command, "vp release --yes");
}

#[test]
fn validate_release_options_rejects_real_skip_publish() {
    let error = validate_release_options(&ReleaseOptions {
        dry_run: false,
        skip_publish: true,
        first_release: false,
        changelog: false,
        version: None,
        preid: None,
        otp: None,
        projects: None,
        git_tag: true,
        git_commit: true,
        run_checks: true,
        yes: false,
    })
    .unwrap_err();

    assert!(error.to_string().contains("--skip-publish"));
}

#[test]
fn validate_release_options_rejects_real_no_git_tag() {
    let error = validate_release_options(&ReleaseOptions {
        dry_run: false,
        skip_publish: false,
        first_release: false,
        changelog: false,
        version: None,
        preid: None,
        otp: None,
        projects: None,
        git_tag: false,
        git_commit: true,
        run_checks: true,
        yes: false,
    })
    .unwrap_err();

    assert!(error.to_string().contains("--no-git-tag"));
}

#[test]
fn validate_release_options_rejects_real_git_tag_without_commit() {
    let error = validate_release_options(&ReleaseOptions {
        dry_run: false,
        skip_publish: false,
        first_release: false,
        changelog: false,
        version: None,
        preid: None,
        otp: None,
        projects: None,
        git_tag: true,
        git_commit: false,
        run_checks: true,
        yes: false,
    })
    .unwrap_err();

    assert!(error.to_string().contains("--no-git-commit"));
    assert!(error.to_string().contains("--git-tag"));
}

#[test]
fn validate_release_options_allows_preview_only_flags_in_dry_run() {
    assert!(
        validate_release_options(&ReleaseOptions {
            dry_run: true,
            skip_publish: true,
            first_release: true,
            changelog: true,
            version: None,
            preid: Some("beta".into()),
            otp: None,
            projects: Some(vec!["pkg-a".into()]),
            git_tag: false,
            git_commit: false,
            run_checks: false,
            yes: false,
        })
        .is_ok()
    );
}

#[test]
fn validate_release_options_rejects_version_with_preid() {
    let error = validate_release_options(&ReleaseOptions {
        dry_run: false,
        skip_publish: false,
        first_release: false,
        changelog: false,
        version: Some("1.2.3-alpha.0".into()),
        preid: Some("alpha".into()),
        otp: None,
        projects: None,
        git_tag: true,
        git_commit: true,
        run_checks: true,
        yes: false,
    })
    .unwrap_err();

    assert!(error.to_string().contains("--version"));
    assert!(error.to_string().contains("--preid"));
}

#[test]
fn resolved_publish_tag_prefers_cli_preid_over_manifest_tag() {
    let mut plan = make_release_plan("pkg-a", &[], &[]);
    plan.publish_tag = Some("next".into());

    let options = ReleaseOptions {
        dry_run: false,
        skip_publish: false,
        first_release: false,
        changelog: false,
        version: None,
        preid: Some("beta".into()),
        otp: None,
        projects: None,
        git_tag: true,
        git_commit: true,
        run_checks: true,
        yes: false,
    };

    assert_eq!(resolved_publish_tag(&plan, &options), Some("beta"));
}

#[test]
fn resolved_publish_tag_falls_back_to_manifest_tag() {
    let mut plan = make_release_plan("pkg-a", &[], &[]);
    plan.publish_tag = Some("next".into());

    assert_eq!(resolved_publish_tag(&plan, &make_release_options()), Some("next"));
}

#[test]
fn build_manifest_edits_updates_simple_internal_dependency_ranges() {
    let mut pkg_a = make_release_plan("pkg-a", &[], &[]);
    pkg_a.current_version = Version::parse("1.0.0").unwrap();
    pkg_a.next_version = Version::parse("1.1.0").unwrap();
    pkg_a.tag_name = "release/pkg-a/v1.1.0".into();
    pkg_a.manifest.version = "1.0.0".into();
    pkg_a.manifest_contents = r#"{
  "name": "pkg-a",
  "version": "1.0.0"
}
"#
    .into();

    let mut pkg_b = make_release_plan("pkg-b", &[], &[]);
    pkg_b.current_version = Version::parse("1.0.0").unwrap();
    pkg_b.next_version = Version::parse("1.0.1").unwrap();
    pkg_b.tag_name = "release/pkg-b/v1.0.1".into();
    pkg_b.manifest.version = "1.0.0".into();
    pkg_b.manifest.dependencies.insert("pkg-a".into(), "^1.0.0".into());
    pkg_b.manifest_contents = r#"{
  "name": "pkg-b",
  "version": "1.0.0",
  "dependencies": {
    "pkg-a": "^1.0.0"
  }
}
"#
    .into();

    let edits = build_manifest_edits(&[pkg_a, pkg_b]).unwrap();
    assert_eq!(edits.len(), 2);
    assert!(edits[1].updated_contents.contains(r#""pkg-a": "^1.1.0""#));
    assert!(edits[1].updated_contents.contains(r#""version": "1.0.1""#));
}

#[test]
fn build_manifest_edits_rejects_complex_internal_dependency_ranges() {
    let mut pkg_a = make_release_plan("pkg-a", &[], &[]);
    pkg_a.current_version = Version::parse("1.0.0").unwrap();
    pkg_a.next_version = Version::parse("1.1.0").unwrap();
    pkg_a.tag_name = "release/pkg-a/v1.1.0".into();
    pkg_a.manifest.version = "1.0.0".into();

    let mut pkg_b = make_release_plan("pkg-b", &[], &[]);
    pkg_b.manifest.dependencies.insert("pkg-a".into(), ">=1.0.0 <2.0.0".into());

    let error = build_manifest_edits(&[pkg_a, pkg_b]).unwrap_err();
    assert!(error.to_string().contains("Use `workspace:` or a simple exact/^/~ version"));
}

#[test]
fn readiness_report_collects_workspace_and_package_scripts() {
    let workspace_manifest: PackageManifest = serde_json::from_str(
        r#"{
            "scripts": {
                "build": "pnpm -r build",
                "release:verify": "pnpm test"
            },
            "vitePlus": {
                "release": {
                    "checkScripts": ["release:verify"]
                }
            }
        }"#,
    )
    .unwrap();
    let plans = vec![make_release_plan("pkg-a", &["build", "prepack"], &[])];

    let report = collect_release_readiness_report(
        Some(&workspace_manifest),
        &plans,
        &make_release_options(),
        &github_hosted_trusted_publish_context(),
    );
    assert_eq!(report.workspace_scripts, vec!["build", "release:verify"]);
    assert_eq!(report.package_scripts.len(), 1);
    assert_eq!(report.package_scripts[0].package, "pkg-a");
    assert_eq!(report.package_scripts[0].scripts, vec!["build", "prepack"]);
    assert!(report.warnings.is_empty());
}

#[test]
fn readiness_report_warns_for_missing_custom_scripts() {
    let plans = vec![make_release_plan("pkg-a", &["build"], &["release:verify"])];

    let report = collect_release_readiness_report(
        None,
        &plans,
        &make_release_options(),
        &github_hosted_trusted_publish_context(),
    );

    assert_eq!(report.package_scripts[0].scripts, vec!["build"]);
    assert_eq!(report.warnings.len(), 1);
    assert!(report.warnings[0].contains("release:verify"));
}

#[test]
fn readiness_report_warns_when_no_obvious_checks_exist() {
    let plans = vec![make_release_plan("pkg-a", &[], &[])];

    let report = collect_release_readiness_report(
        None,
        &plans,
        &make_release_options(),
        &github_hosted_trusted_publish_context(),
    );

    assert!(report.package_scripts.is_empty());
    assert_eq!(report.warnings.len(), 2);
    assert!(report.warnings[0].contains("pkg-a"));
    assert!(report.warnings[1].contains("No explicit build / pack / prepack"));
}

#[test]
fn classify_conventional_commits() {
    assert_eq!(classify_commit("feat(cli): add release", ""), Some(VersionBump::Minor));
    assert_eq!(classify_commit("fix(core): handle null", ""), Some(VersionBump::Patch));
    assert_eq!(classify_commit("fix!: break behavior", ""), Some(VersionBump::Major));
    assert_eq!(
        classify_commit("chore: cleanup", "BREAKING CHANGE: changed the API"),
        Some(VersionBump::Major)
    );
    assert_eq!(classify_commit("docs: update readme", ""), None);
}

#[test]
fn bump_version_supports_prerelease() {
    let current = Version::parse("1.2.3").unwrap();
    assert_eq!(
        next_release_version(&current, VersionBump::Patch, None, None).unwrap().to_string(),
        "1.2.4"
    );
    assert_eq!(
        next_release_version(&current, VersionBump::Minor, None, Some("beta")).unwrap().to_string(),
        "1.3.0-beta.0"
    );
}

#[test]
fn zero_major_breaking_changes_are_downgraded_to_minor_bumps() {
    let current = Version::parse("0.4.2").unwrap();
    assert_eq!(effective_release_level(&current, VersionBump::Major), VersionBump::Minor);
    assert_eq!(
        next_release_version(
            &current,
            effective_release_level(&current, VersionBump::Major),
            None,
            None
        )
        .unwrap()
        .to_string(),
        "0.5.0"
    );
}

#[test]
fn stable_major_versions_keep_breaking_changes_as_major_bumps() {
    let current = Version::parse("1.4.2").unwrap();
    assert_eq!(effective_release_level(&current, VersionBump::Major), VersionBump::Major);
}

#[test]
fn prerelease_on_same_channel_increments_sequence() {
    let stable = Version::parse("1.0.0").unwrap();
    let current = Version::parse("1.1.0-alpha.0").unwrap();
    assert_eq!(
        next_release_version(&current, VersionBump::Patch, Some(&stable), Some("alpha"))
            .unwrap()
            .to_string(),
        "1.1.0-alpha.1"
    );
}

#[test]
fn prerelease_can_escalate_to_higher_release_line() {
    let stable = Version::parse("1.0.0").unwrap();
    let current = Version::parse("1.0.1-alpha.0").unwrap();
    assert_eq!(
        next_release_version(&current, VersionBump::Minor, Some(&stable), Some("alpha"))
            .unwrap()
            .to_string(),
        "1.1.0-alpha.0"
    );
}

#[test]
fn prerelease_can_switch_channels_without_bumping_base_again() {
    let stable = Version::parse("1.0.0").unwrap();
    let current = Version::parse("1.1.0-alpha.2").unwrap();
    assert_eq!(
        next_release_version(&current, VersionBump::Patch, Some(&stable), Some("beta"))
            .unwrap()
            .to_string(),
        "1.1.0-beta.0"
    );
}

#[test]
fn prerelease_supports_rc_channel() {
    let stable = Version::parse("1.0.0").unwrap();
    let current = Version::parse("1.1.0-beta.2").unwrap();
    assert_eq!(
        next_release_version(&current, VersionBump::Patch, Some(&stable), Some("rc"))
            .unwrap()
            .to_string(),
        "1.1.0-rc.0"
    );
    let current = Version::parse("1.1.0-rc.0").unwrap();
    assert_eq!(
        next_release_version(&current, VersionBump::Patch, Some(&stable), Some("rc"))
            .unwrap()
            .to_string(),
        "1.1.0-rc.1"
    );
}

#[test]
fn stable_release_from_prerelease_keeps_current_target_version() {
    let stable = Version::parse("1.0.0").unwrap();
    let current = Version::parse("1.1.0-beta.2").unwrap();
    assert_eq!(
        next_release_version(&current, VersionBump::Patch, Some(&stable), None)
            .unwrap()
            .to_string(),
        "1.1.0"
    );
}

#[test]
fn zero_major_prerelease_breaking_changes_stay_on_minor_line() {
    let stable = Version::parse("0.1.0").unwrap();
    let current = Version::parse("0.2.0-alpha.1").unwrap();
    let level = effective_release_level(&current, VersionBump::Major);
    assert_eq!(level, VersionBump::Minor);
    assert_eq!(
        next_release_version(&current, level, Some(&stable), Some("alpha")).unwrap().to_string(),
        "0.2.0-alpha.2"
    );
}

#[test]
fn prerelease_levels_cannot_be_used_as_bump_levels() {
    let current = Version::parse("1.0.0").unwrap();
    let error = next_release_version(&current, VersionBump::Alpha, None, None).unwrap_err();
    assert!(error.to_string().contains("Invalid release level 'alpha'"));
}

#[test]
fn replace_top_level_string_property_only_updates_top_level_field() {
    let contents = r#"{
  "version": "1.0.0",
  "nested": {
    "version": "should-stay"
  }
}
"#;

    let updated = replace_top_level_string_property(contents, "version", "1.0.0", "2.0.0").unwrap();
    assert!(updated.contains(r#""version": "2.0.0""#));
    assert!(updated.contains(r#""version": "should-stay""#));
}

#[test]
fn prepend_changelog_section_reuses_existing_heading() {
    let existing = "# Changelog\n\n## 0.1.0 - 2026-01-01\n\n- existing\n";
    let prepended = prepend_changelog_section(existing, "## 0.2.0 - 2026-02-01\n\n- new\n\n");
    assert!(prepended.starts_with("# Changelog\n\n## 0.2.0 - 2026-02-01"));
    assert!(prepended.contains("## 0.1.0 - 2026-01-01"));
}

#[test]
fn prepend_changelog_section_adds_heading_when_missing() {
    let existing = "## 0.1.0 - 2026-01-01\n\n- existing\n";
    let prepended = prepend_changelog_section(existing, "## 0.2.0 - 2026-02-01\n\n- new\n\n");
    assert!(prepended.starts_with("# Changelog\n\n## 0.2.0 - 2026-02-01"));
    assert!(prepended.contains("## 0.1.0 - 2026-01-01"));
}

#[test]
fn prepend_changelog_section_handles_heading_without_existing_entries() {
    let prepended =
        prepend_changelog_section("# Changelog\n", "## 0.2.0 - 2026-02-01\n\n- new\n\n");
    assert_eq!(prepended, "# Changelog\n\n## 0.2.0 - 2026-02-01\n\n- new\n\n");
}

#[test]
fn summarize_release_artifacts_counts_manifests_and_changelogs() {
    let plans = vec![make_release_plan("pkg-a", &[], &[]), make_release_plan("pkg-b", &[], &[])];
    let manifest_edits = vec![
        ManifestEdit {
            package: "pkg-a".into(),
            path: test_absolute_path("/packages/pkg-a/package.json"),
            original_contents: "{}".into(),
            updated_contents: r#"{"version":"1.0.1"}"#.into(),
        },
        ManifestEdit {
            package: "pkg-b".into(),
            path: test_absolute_path("/packages/pkg-b/package.json"),
            original_contents: "{}".into(),
            updated_contents: r#"{"version":"1.0.1"}"#.into(),
        },
    ];

    let summary = summarize_release_artifacts(&plans, &manifest_edits, true);
    assert_eq!(summary.manifest_file_count, 2);
    assert_eq!(summary.changelog_file_count, 3);
    assert_eq!(summary.total_file_count(), 5);
}

#[test]
fn rollback_release_artifact_edits_restores_and_removes_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace_root = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

    let existing_path = workspace_root.join("existing.txt");
    std::fs::write(&existing_path, "before").unwrap();
    let generated_path = workspace_root.join("generated.txt");
    std::fs::write(&generated_path, "after").unwrap();

    let edits = vec![
        ReleaseArtifactEdit {
            label: "existing file".into(),
            path: existing_path.clone(),
            original_contents: Some(String::from("before")),
            updated_contents: String::from("after"),
        },
        ReleaseArtifactEdit {
            label: "generated file".into(),
            path: generated_path.clone(),
            original_contents: None,
            updated_contents: String::from("after"),
        },
    ];

    rollback_release_artifact_edits(&edits).unwrap();

    assert_eq!(std::fs::read_to_string(&existing_path).unwrap(), "before");
    assert!(!generated_path.as_path().exists());
}

#[test]
fn trusted_publish_context_detects_github_actions_hosted_runner() {
    let context = github_hosted_trusted_publish_context();

    assert!(context.supports_trusted_publishing());
    assert!(context.supports_publish_provenance());
    assert_eq!(context.workflow_path().as_deref(), Some(".github/workflows/release.yml"));
    assert_eq!(context.repository.as_deref(), Some("voidzero-dev/vite-plus"));
}

#[test]
fn trusted_publish_context_detects_gitlab_ci() {
    let context = gitlab_trusted_publish_context();

    assert!(context.supports_trusted_publishing());
    assert!(context.supports_publish_provenance());
    assert!(context.workflow_path().is_none());
    assert!(context.environment_summary().contains("GitLab CI"));
}

#[test]
fn trusted_publish_context_detects_circleci_without_provenance() {
    let context = circleci_trusted_publish_context();

    assert!(context.supports_trusted_publishing());
    assert!(!context.supports_publish_provenance());
    assert!(context.environment_summary().contains("CircleCI"));
}

#[test]
fn trusted_publish_context_parses_publish_workflow_ref() {
    let context = TrustedPublishContext::from_env(|key| match key {
        "GITHUB_ACTIONS" => Some(String::from("true")),
        "RUNNER_ENVIRONMENT" => Some(String::from("github-hosted")),
        "GITHUB_WORKFLOW_REF" => Some(String::from(
            "voidzero-dev/vite-plus/.github/workflows/publish.yml@refs/heads/main",
        )),
        _ => None,
    });

    assert_eq!(context.workflow_path().as_deref(), Some(".github/workflows/publish.yml"));
}

#[test]
fn trusted_publish_context_returns_no_workflow_path_for_unexpected_ref_shape() {
    let context = TrustedPublishContext::from_env(|key| match key {
        "GITHUB_ACTIONS" => Some(String::from("true")),
        "RUNNER_ENVIRONMENT" => Some(String::from("github-hosted")),
        "GITHUB_WORKFLOW_REF" => Some(String::from("voidzero-dev/vite-plus@refs/heads/main")),
        _ => None,
    });

    assert!(context.workflow_path().is_none());
}

#[test]
fn trusted_publish_context_rejects_github_actions_self_hosted_runner() {
    let context = TrustedPublishContext::from_env(|key| match key {
        "GITHUB_ACTIONS" => Some(String::from("true")),
        "RUNNER_ENVIRONMENT" => Some(String::from("self-hosted")),
        _ => None,
    });

    assert!(!context.supports_trusted_publishing());
    assert!(!context.supports_publish_provenance());
}

#[test]
fn validate_trusted_publish_context_rejects_local_real_release() {
    let error = validate_trusted_publish_context(
        &make_release_options(),
        &TrustedPublishContext::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("trusted-publishing CI"));
}

#[test]
fn validate_trusted_publish_context_allows_local_dry_run() {
    let mut options = make_release_options();
    options.dry_run = true;

    assert!(validate_trusted_publish_context(&options, &TrustedPublishContext::default()).is_ok());
}

#[test]
fn validate_trusted_publish_context_rejects_circleci_without_provenance() {
    let context = circleci_trusted_publish_context();

    let error = validate_trusted_publish_context(&make_release_options(), &context).unwrap_err();
    assert!(error.to_string().contains("provenance attestations"));
}

#[test]
fn validate_trusted_publish_context_allows_gitlab_real_release() {
    assert!(
        validate_trusted_publish_context(
            &make_release_options(),
            &gitlab_trusted_publish_context()
        )
        .is_ok()
    );
}

#[test]
fn resolved_publish_provenance_defaults_to_true_in_trusted_publish_ci() {
    let plan = make_release_plan("pkg-a", &[], &[]);
    let context = github_hosted_trusted_publish_context();

    assert_eq!(resolved_publish_provenance(&plan, &context), Some(true));
}

#[test]
fn resolved_publish_provenance_respects_explicit_opt_out() {
    let mut plan = make_release_plan("pkg-a", &[], &[]);
    plan.publish_provenance = Some(false);

    assert_eq!(
        resolved_publish_provenance(&plan, &github_hosted_trusted_publish_context()),
        Some(false)
    );
}

#[test]
fn resolved_publish_provenance_stays_none_without_capable_environment() {
    let plan = make_release_plan("pkg-a", &[], &[]);
    assert_eq!(resolved_publish_provenance(&plan, &TrustedPublishContext::default()), None);
}

#[test]
fn readiness_report_tracks_provenance_opt_out_and_legacy_otp() {
    let mut options = make_release_options();
    options.otp = Some(String::from("123456"));
    let mut plan = make_release_plan("pkg-a", &["build"], &[]);
    plan.publish_provenance = Some(false);

    let report = collect_release_readiness_report(
        None,
        &[plan],
        &options,
        &github_hosted_trusted_publish_context(),
    );

    assert_eq!(report.trusted_publish.packages_with_provenance_disabled, vec!["pkg-a"]);
    assert!(report.trusted_publish.uses_legacy_otp);
}

#[test]
fn readiness_report_warns_when_environment_cannot_emit_provenance() {
    let report = collect_release_readiness_report(
        None,
        &[make_release_plan("pkg-a", &["build"], &[])],
        &make_release_options(),
        &TrustedPublishContext::default(),
    );

    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("cannot produce the npm provenance attestations"))
    );
}

#[test]
fn package_tags_are_scoped_and_safe() {
    let version = Version::parse("1.0.0").unwrap();
    assert_eq!(package_tag_name("@scope/pkg-a", &version), "release/scope/pkg-a/v1.0.0");
}

#[test]
fn release_tags_roundtrip_scoped_and_unscoped_package_names() {
    assert_eq!(
        parse_package_name_from_release_tag("release/scope/pkg-a/v1.0.0"),
        Some("@scope/pkg-a".into())
    );
    assert_eq!(parse_package_name_from_release_tag("release/pkg-b/v2.0.0"), Some("pkg-b".into()));
}

#[test]
fn invalid_release_tags_are_ignored() {
    assert_eq!(parse_package_name_from_release_tag("release//v1.0.0"), None);
    assert_eq!(parse_package_name_from_release_tag("release/pkg-a/not-a-version"), None);
    assert_eq!(parse_package_name_from_release_tag("pkg-a@1.0.0"), None);
}

#[test]
fn repository_release_tags_accept_stable_and_standard_prereleases_only() {
    assert_eq!(
        parse_repository_release_tag_version_for_tests("v1.2.3").unwrap().to_string(),
        "1.2.3"
    );
    assert_eq!(
        parse_repository_release_tag_version_for_tests("v1.2.4-alpha.1").unwrap().to_string(),
        "1.2.4-alpha.1"
    );
    assert!(parse_repository_release_tag_version_for_tests("v0.0.0-16aec32").is_none());
    assert!(parse_repository_release_tag_version_for_tests("release/pkg-a/v1.0.0").is_none());
}

#[test]
fn publish_protocol_matrix_prefers_native_workspace_and_catalog_rewrites() {
    let pnpm = test_package_manager(PackageManagerType::Pnpm, "10.0.0");
    let summary =
        DependencyProtocolSummary { workspace: true, catalog: true, ..Default::default() };

    assert!(unsupported_publish_protocols(&pnpm, summary).is_empty());

    let npm = test_package_manager(PackageManagerType::Npm, "11.0.0");
    assert_eq!(unsupported_publish_protocols(&npm, summary), vec!["workspace:", "catalog:"]);
}

#[test]
fn publish_protocol_matrix_allows_workspace_and_catalog_for_bun() {
    let bun = test_package_manager(PackageManagerType::Bun, "1.2.0");
    let summary =
        DependencyProtocolSummary { workspace: true, catalog: true, ..Default::default() };

    assert!(unsupported_publish_protocols(&bun, summary).is_empty());
}

#[test]
fn publish_protocol_matrix_rejects_workspace_and_catalog_for_yarn1() {
    let yarn = test_package_manager(PackageManagerType::Yarn, "1.22.0");
    let summary =
        DependencyProtocolSummary { workspace: true, catalog: true, ..Default::default() };

    assert_eq!(unsupported_publish_protocols(&yarn, summary), vec!["workspace:", "catalog:"]);
}

#[test]
fn file_protocol_remains_blocked_even_with_native_publishers() {
    let pnpm = test_package_manager(PackageManagerType::Pnpm, "10.0.0");
    let summary = DependencyProtocolSummary { file: true, ..Default::default() };
    assert_eq!(unsupported_publish_protocols(&pnpm, summary), vec!["file:"]);
}

#[test]
fn project_selection_matches_previous_package_names() {
    let graph = build_test_package_graph();
    let mut nodes = graph.node_indices().filter(|&node| !graph[node].path.as_str().is_empty());
    let pkg_a = nodes.next().unwrap();

    let mut package = make_workspace_package(&graph, pkg_a, usize::MAX);
    package.known_names.push("@scope/old-pkg-a".into());

    let selected =
        select_workspace_packages(&[package], Some(&["@scope/old-pkg-*".into()])).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].name, "pkg-a");
}

#[test]
fn release_commit_message_lists_up_to_three_packages() {
    let plans = vec![
        make_release_plan("pkg-a", &[], &[]),
        make_release_plan("pkg-b", &[], &[]),
        make_release_plan("pkg-c", &[], &[]),
    ];

    assert_eq!(
        release_commit_message(&plans),
        "chore(release): publish pkg-a@1.0.1, pkg-b@1.0.1, pkg-c@1.0.1"
    );
}

#[test]
fn release_commit_message_summarizes_larger_release_sets() {
    let plans = vec![
        make_release_plan("pkg-a", &[], &[]),
        make_release_plan("pkg-b", &[], &[]),
        make_release_plan("pkg-c", &[], &[]),
        make_release_plan("pkg-d", &[], &[]),
    ];

    assert_eq!(release_commit_message(&plans), "chore(release): publish 4 packages");
}

#[test]
fn unique_strings_preserves_order() {
    let values =
        unique_strings(vec!["a".to_string(), "b".to_string(), "a".to_string(), "c".to_string()]);
    assert_eq!(values, vec!["a", "b", "c"]);
}

#[test]
fn orphaned_released_packages_ignore_known_and_retired_names() {
    let known = HashSet::from(["pkg-a", "@scope/pkg-b", "@scope/old-pkg-c"]);
    let orphaned = collect_orphaned_released_package_names(
        [
            "release/pkg-a/v1.0.0",
            "release/scope/pkg-b/v1.0.0",
            "release/scope/old-pkg-c/v0.9.0",
            "release/scope/pkg-d/v2.0.0",
        ],
        &known,
    );
    assert_eq!(orphaned, vec!["@scope/pkg-d"]);
}

#[test]
fn orphaned_released_packages_ignore_invalid_tag_shapes() {
    let known = HashSet::from(["pkg-a"]);
    let orphaned = collect_orphaned_released_package_names(
        ["not-a-tag", "release//v1.0.0", "release/pkg-a/not-a-version"],
        &known,
    );
    assert!(orphaned.is_empty());
}

#[test]
fn project_order_breaks_ties_between_independent_packages() {
    let graph = build_test_package_graph();
    let mut nodes = graph.node_indices().filter(|&node| !graph[node].path.as_str().is_empty());
    let pkg_a = nodes.next().unwrap();
    let pkg_b = nodes.next().unwrap();
    let pkg_c = nodes.next().unwrap();

    let selected = vec![
        make_workspace_package(&graph, pkg_a, 2),
        make_workspace_package(&graph, pkg_b, 1),
        make_workspace_package(&graph, pkg_c, 0),
    ];

    let ordered = topological_sort_selected_packages(&graph, &selected);
    let names: Vec<&str> = ordered.iter().map(|package| package.name.as_str()).collect();

    assert_eq!(names, vec!["pkg-b", "pkg-c", "pkg-a"]);
}

#[test]
fn dependency_order_wins_over_requested_project_order() {
    let graph = build_test_package_graph();
    let mut nodes = graph.node_indices().filter(|&node| !graph[node].path.as_str().is_empty());
    let pkg_a = nodes.next().unwrap();
    let pkg_b = nodes.next().unwrap();

    let selected =
        vec![make_workspace_package(&graph, pkg_a, 0), make_workspace_package(&graph, pkg_b, 1)];

    let ordered = topological_sort_selected_packages(&graph, &selected);
    let names: Vec<&str> = ordered.iter().map(|package| package.name.as_str()).collect();

    assert_eq!(names, vec!["pkg-b", "pkg-a"]);
}

#[test]
fn cycle_breaker_prefers_selection_order_for_cycles() {
    let mut graph = build_test_package_graph();
    let mut nodes = graph.node_indices().filter(|&node| !graph[node].path.as_str().is_empty());
    let pkg_a = nodes.next().unwrap();
    let pkg_b = nodes.next().unwrap();

    graph.add_edge(pkg_b, pkg_a, DependencyType::Normal);

    let selected =
        vec![make_workspace_package(&graph, pkg_a, 1), make_workspace_package(&graph, pkg_b, 0)];

    let ordered = topological_sort_selected_packages(&graph, &selected);
    let names: Vec<&str> = ordered.iter().map(|package| package.name.as_str()).collect();

    assert_eq!(names, vec!["pkg-b", "pkg-a"]);
}

#[test]
fn cycle_breaker_uses_package_name_when_selection_order_matches() {
    let mut graph = build_test_package_graph();
    let mut nodes = graph.node_indices().filter(|&node| !graph[node].path.as_str().is_empty());
    let pkg_a = nodes.next().unwrap();
    let pkg_b = nodes.next().unwrap();

    graph.add_edge(pkg_b, pkg_a, DependencyType::Normal);

    let selected =
        vec![make_workspace_package(&graph, pkg_a, 0), make_workspace_package(&graph, pkg_b, 0)];

    let ordered = topological_sort_selected_packages(&graph, &selected);
    let names: Vec<&str> = ordered.iter().map(|package| package.name.as_str()).collect();

    assert_eq!(names, vec!["pkg-a", "pkg-b"]);
}
