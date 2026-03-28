//! Publish command resolution for npm-compatible package managers.
//!
//! The release flow delegates publish execution to this module so that package-manager-specific
//! argument quirks, environment wiring, and provenance handling stay centralized. The output of
//! this module is intentionally explicit: a resolved binary, argument list, and environment map
//! that higher-level callers can inspect during dry-runs or execute during real publishes.

use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the publish command.
///
/// This structure describes publish intent in package-manager-neutral terms. Resolution into a
/// concrete CLI command happens later in `PackageManager::resolve_publish_command`.
#[derive(Debug, Default)]
pub struct PublishCommandOptions<'a> {
    /// Optional tarball/directory target passed to the underlying publisher.
    pub target: Option<&'a str>,
    /// Whether the resolved command should only simulate publishing.
    pub dry_run: bool,
    /// Dist-tag to publish under.
    pub tag: Option<&'a str>,
    /// Access level, typically `public` for first publish of scoped public packages.
    pub access: Option<&'a str>,
    /// Legacy TOTP code for npm 2FA flows.
    pub otp: Option<&'a str>,
    /// npm provenance preference propagated through environment configuration when supported.
    pub provenance: Option<bool>,
    /// Disables git checks when the package manager supports it.
    pub no_git_checks: bool,
    /// Branch restriction for publishers that support release branches.
    pub publish_branch: Option<&'a str>,
    /// Requests a publish summary file/output when supported.
    pub report_summary: bool,
    /// Forces publish when the package manager exposes such a flag.
    pub force: bool,
    /// Requests machine-readable JSON output when supported.
    pub json: bool,
    /// Enables recursive/workspace publish mode when supported.
    pub recursive: bool,
    /// Package-manager-native workspace filters.
    pub filters: Option<&'a [String]>,
    /// Reserved passthrough arguments for future publish extensions.
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Runs the resolved publish command with the package manager.
    #[must_use]
    pub async fn run_publish_command(
        &self,
        options: &PublishCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_publish_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolves the publish command into a concrete executable, argument vector, and env map.
    ///
    /// Prefer native publish commands when they provide better protocol handling, especially for
    /// workspace/catalog-style references that may need publisher-specific rewriting.
    #[must_use]
    pub fn resolve_publish_command(&self, options: &PublishCommandOptions) -> ResolveCommandResult {
        let mut envs = HashMap::with_capacity(2);
        envs.insert(String::from("PATH"), format_path_env(self.get_bin_prefix()));
        if let Some(provenance) = options.provenance {
            envs.insert(
                String::from("NPM_CONFIG_PROVENANCE"),
                if provenance { String::from("true") } else { String::from("false") },
            );
        }
        let mut args: Vec<String> = Vec::with_capacity(16);

        let bin_name: String;

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();

                // pnpm treats filtering as a global option, so it must appear before `publish`.
                // https://pnpm.io/cli/publish
                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--filter".into());
                        args.push(filter.clone());
                    }
                }

                args.push("publish".into());

                if let Some(target) = options.target {
                    args.push(target.to_string());
                }

                if options.dry_run {
                    args.push("--dry-run".into());
                }

                if let Some(tag) = options.tag {
                    args.push("--tag".into());
                    args.push(tag.to_string());
                }

                if let Some(access) = options.access {
                    args.push("--access".into());
                    args.push(access.to_string());
                }

                if let Some(otp) = options.otp {
                    args.push("--otp".into());
                    args.push(otp.to_string());
                }

                if options.no_git_checks {
                    args.push("--no-git-checks".into());
                }

                if let Some(branch) = options.publish_branch {
                    args.push("--publish-branch".into());
                    args.push(branch.to_string());
                }

                if options.report_summary {
                    args.push("--report-summary".into());
                }

                if options.force {
                    args.push("--force".into());
                }

                if options.json {
                    args.push("--json".into());
                }

                if options.recursive {
                    args.push("--recursive".into());
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();

                args.push("publish".into());

                // npm workspace selection is expressed with per-workspace flags after the command.
                // https://docs.npmjs.com/cli/v11/commands/npm-publish
                // https://docs.npmjs.com/cli/v11/using-npm/workspaces/
                if options.recursive {
                    args.push("--workspaces".into());
                }

                if let Some(filters) = options.filters {
                    for filter in filters {
                        args.push("--workspace".into());
                        args.push(filter.clone());
                    }
                }

                if let Some(target) = options.target {
                    args.push(target.to_string());
                }

                if options.dry_run {
                    args.push("--dry-run".into());
                }

                if let Some(tag) = options.tag {
                    args.push("--tag".into());
                    args.push(tag.to_string());
                }

                if let Some(access) = options.access {
                    args.push("--access".into());
                    args.push(access.to_string());
                }

                if let Some(otp) = options.otp {
                    args.push("--otp".into());
                    args.push(otp.to_string());
                }

                if options.force {
                    args.push("--force".into());
                }

                if options.publish_branch.is_some() {
                    output::warn("--publish-branch not supported by npm, ignoring flag");
                }

                if options.report_summary {
                    output::warn("--report-summary not supported by npm, ignoring flag");
                }

                if options.json {
                    output::warn("--json not supported by npm, ignoring flag");
                }
            }
            PackageManagerType::Yarn => {
                // Modern Yarn has its own publish surface (`yarn npm publish`), while Yarn 1 falls
                // back to npm semantics. Keep the native path when it preserves documented behavior.
                // https://yarnpkg.com/cli/npm/publish
                let can_use_native_yarn = !self.version.starts_with("1.")
                    && options.target.is_none()
                    && !options.recursive
                    && options.filters.map_or(true, |filters| filters.is_empty())
                    && options.publish_branch.is_none()
                    && !options.report_summary
                    && !options.force;

                if can_use_native_yarn {
                    bin_name = "yarn".into();
                    args.push("npm".into());
                    args.push("publish".into());

                    if options.dry_run {
                        args.push("--dry-run".into());
                    }

                    if let Some(tag) = options.tag {
                        args.push("--tag".into());
                        args.push(tag.to_string());
                    }

                    if let Some(access) = options.access {
                        args.push("--access".into());
                        args.push(access.to_string());
                    }

                    if let Some(otp) = options.otp {
                        args.push("--otp".into());
                        args.push(otp.to_string());
                    }

                    if options.no_git_checks {
                        output::warn(
                            "--no-git-checks not supported by yarn npm publish, ignoring flag",
                        );
                    }

                    if options.json {
                        args.push("--json".into());
                    }
                } else {
                    bin_name = "npm".into();

                    args.push("publish".into());

                    if options.recursive {
                        args.push("--workspaces".into());
                    }

                    if let Some(filters) = options.filters {
                        for filter in filters {
                            args.push("--workspace".into());
                            args.push(filter.clone());
                        }
                    }

                    if let Some(target) = options.target {
                        args.push(target.to_string());
                    }

                    if options.dry_run {
                        args.push("--dry-run".into());
                    }

                    if let Some(tag) = options.tag {
                        args.push("--tag".into());
                        args.push(tag.to_string());
                    }

                    if let Some(access) = options.access {
                        args.push("--access".into());
                        args.push(access.to_string());
                    }

                    if let Some(otp) = options.otp {
                        args.push("--otp".into());
                        args.push(otp.to_string());
                    }

                    if options.force {
                        args.push("--force".into());
                    }

                    if options.publish_branch.is_some() {
                        output::warn(
                            "--publish-branch not supported by yarn native publish flow, falling back to npm publish",
                        );
                    }

                    if options.report_summary {
                        output::warn(
                            "--report-summary not supported by yarn native publish flow, falling back to npm publish",
                        );
                    }

                    if options.json {
                        output::warn("--json not supported by npm, ignoring flag");
                    }
                }
            }
            PackageManagerType::Bun => {
                // Bun exposes its own `publish` command, but does not currently mirror npm/pnpm's
                // workspace/filter feature set, so unsupported flags are surfaced explicitly.
                // https://bun.sh/docs/pm/cli/publish
                bin_name = "bun".into();

                args.push("publish".into());

                if let Some(target) = options.target {
                    args.push(target.to_string());
                }

                if options.dry_run {
                    args.push("--dry-run".into());
                }

                if let Some(tag) = options.tag {
                    args.push("--tag".into());
                    args.push(tag.to_string());
                }

                if let Some(access) = options.access {
                    args.push("--access".into());
                    args.push(access.to_string());
                }

                if let Some(otp) = options.otp {
                    args.push("--otp".into());
                    args.push(otp.to_string());
                }

                if options.no_git_checks {
                    output::warn("--no-git-checks not supported by bun, ignoring flag");
                }

                if options.publish_branch.is_some() {
                    output::warn("--publish-branch not supported by bun, ignoring flag");
                }

                if options.report_summary {
                    output::warn("--report-summary not supported by bun, ignoring flag");
                }

                if options.force {
                    output::warn("--force not supported by bun publish, ignoring flag");
                }

                if options.json {
                    output::warn("--json not supported by bun publish, ignoring flag");
                }

                if options.recursive {
                    output::warn("--recursive not supported by bun publish, ignoring flag");
                }

                if let Some(filters) = options.filters {
                    if !filters.is_empty() {
                        output::warn("--filter not supported by bun publish, ignoring flag");
                    }
                }
            }
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};
    use vite_path::AbsolutePathBuf;
    use vite_str::Str;

    use super::*;

    fn create_temp_dir() -> TempDir {
        tempdir().expect("Failed to create temp directory")
    }

    fn create_mock_package_manager(pm_type: PackageManagerType, version: &str) -> PackageManager {
        let temp_dir = create_temp_dir();
        let temp_dir_path = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let install_dir = temp_dir_path.join("install");

        PackageManager {
            client: pm_type,
            package_name: pm_type.to_string().into(),
            version: Str::from(version),
            hash: None,
            bin_name: pm_type.to_string().into(),
            workspace_root: temp_dir_path.clone(),
            is_monorepo: false,
            install_dir,
        }
    }

    #[test]
    fn test_pnpm_publish() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_npm_publish() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn1_publish_uses_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn2_publish_uses_yarn_npm_publish() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "publish"]);
    }

    #[test]
    fn test_bun_publish_uses_native_command() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.2.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert_eq!(result.bin_path, "bun");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn_publish_with_tag() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            tag: Some("beta"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "publish", "--tag", "beta"]);
    }

    #[test]
    fn test_pnpm_publish_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--recursive"]);
    }

    #[test]
    fn test_npm_publish_recursive() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--workspaces"]);
    }

    #[test]
    fn test_pnpm_publish_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "publish"]);
    }

    #[test]
    fn test_npm_publish_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--workspace", "app"]);
    }

    #[test]
    fn test_yarn_publish_with_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--workspace", "app"]);
    }

    #[test]
    fn test_yarn_modern_publish_with_filter_falls_back_to_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let filters = vec!["app".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--workspace", "app"]);
    }

    #[test]
    fn test_pnpm_publish_json() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result =
            pm.resolve_publish_command(&PublishCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--json"]);
    }

    #[test]
    fn test_npm_publish_json_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result =
            pm.resolve_publish_command(&PublishCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_pnpm_publish_branch() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            publish_branch: Some("main"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--publish-branch", "main"]);
    }

    #[test]
    fn test_npm_publish_branch_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            publish_branch: Some("main"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_pnpm_publish_report_summary() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            report_summary: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--report-summary"]);
    }

    #[test]
    fn test_npm_publish_report_summary_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            report_summary: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_pnpm_publish_otp() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            otp: Some("123456"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["publish", "--otp", "123456"]);
    }

    #[test]
    fn test_npm_publish_otp() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            otp: Some("654321"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--otp", "654321"]);
    }

    #[test]
    fn test_yarn_publish_otp() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            otp: Some("999999"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--otp", "999999"]);
    }

    #[test]
    fn test_yarn_modern_publish_json_uses_native_command() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result =
            pm.resolve_publish_command(&PublishCommandOptions { json: true, ..Default::default() });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "publish", "--json"]);
    }

    #[test]
    fn test_yarn_modern_publish_with_target_falls_back_to_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            target: Some("./dist/pkg.tgz"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "./dist/pkg.tgz"]);
    }

    #[test]
    fn test_yarn_modern_publish_recursive_falls_back_to_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            recursive: true,
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--workspaces"]);
    }

    #[test]
    fn test_yarn_modern_publish_branch_falls_back_to_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            publish_branch: Some("main"),
            ..Default::default()
        });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish"]);
    }

    #[test]
    fn test_yarn_modern_publish_force_falls_back_to_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm
            .resolve_publish_command(&PublishCommandOptions { force: true, ..Default::default() });
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["publish", "--force"]);
    }

    #[test]
    fn test_pnpm_publish_keeps_filter_order_and_global_position() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let filters = vec!["pkg-b".to_string(), "pkg-a".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&filters),
            dry_run: true,
            recursive: true,
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec!["--filter", "pkg-b", "--filter", "pkg-a", "publish", "--dry-run", "--recursive"]
        );
    }

    #[test]
    fn test_npm_publish_keeps_workspace_filters_after_command() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let filters = vec!["pkg-b".to_string(), "pkg-a".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            filters: Some(&filters),
            recursive: true,
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec!["publish", "--workspaces", "--workspace", "pkg-b", "--workspace", "pkg-a"]
        );
    }

    #[test]
    fn test_pnpm_publish_combines_target_access_and_branch_flags() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            target: Some("./dist/pkg.tgz"),
            dry_run: true,
            tag: Some("beta"),
            access: Some("public"),
            no_git_checks: true,
            publish_branch: Some("main"),
            report_summary: true,
            force: true,
            json: true,
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec![
                "publish",
                "./dist/pkg.tgz",
                "--dry-run",
                "--tag",
                "beta",
                "--access",
                "public",
                "--no-git-checks",
                "--publish-branch",
                "main",
                "--report-summary",
                "--force",
                "--json",
            ]
        );
    }

    #[test]
    fn test_publish_provenance_env_is_enabled() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            provenance: Some(true),
            ..Default::default()
        });
        assert_eq!(result.envs.get("NPM_CONFIG_PROVENANCE").map(String::as_str), Some("true"));
    }

    #[test]
    fn test_publish_provenance_env_can_be_disabled() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            provenance: Some(false),
            ..Default::default()
        });
        assert_eq!(result.envs.get("NPM_CONFIG_PROVENANCE").map(String::as_str), Some("false"));
    }

    #[test]
    fn test_publish_provenance_env_is_absent_when_unspecified() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_publish_command(&PublishCommandOptions::default());
        assert!(!result.envs.contains_key("NPM_CONFIG_PROVENANCE"));
    }

    #[test]
    fn test_bun_publish_keeps_supported_flags_and_ignores_unsupported_ones() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.2.0");
        let filters = vec!["pkg-a".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            target: Some("./dist/pkg.tgz"),
            dry_run: true,
            tag: Some("beta"),
            access: Some("public"),
            otp: Some("123456"),
            no_git_checks: true,
            publish_branch: Some("main"),
            report_summary: true,
            force: true,
            json: true,
            recursive: true,
            filters: Some(&filters),
            ..Default::default()
        });
        assert_eq!(
            result.args,
            vec![
                "publish",
                "./dist/pkg.tgz",
                "--dry-run",
                "--tag",
                "beta",
                "--access",
                "public",
                "--otp",
                "123456",
            ]
        );
    }

    #[test]
    fn test_pass_through_args_are_appended_last() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let pass_through_args = vec!["--provenance-file".to_string(), "out.json".to_string()];
        let result = pm.resolve_publish_command(&PublishCommandOptions {
            dry_run: true,
            pass_through_args: Some(&pass_through_args),
            ..Default::default()
        });
        assert_eq!(result.args, vec!["publish", "--dry-run", "--provenance-file", "out.json"]);
    }
}
