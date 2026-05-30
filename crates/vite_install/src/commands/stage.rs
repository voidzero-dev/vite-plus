use std::{collections::HashMap, process::ExitStatus};

use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Subcommands for the staged-publishing workflow.
///
/// Mirrors npm's `npm stage <publish|list|view|download|approve|reject>` and
/// pnpm's `pnpm stage <…>`. yarn berry reaches the same registry feature
/// through its npm plugin (`yarn npm publish --staged`, `yarn npm stage …`).
#[derive(Debug, Clone)]
pub enum StageSubcommand {
    /// Upload a package to the staging area (no 2FA required).
    Publish {
        target: Option<String>,
        tag: Option<String>,
        access: Option<String>,
        otp: Option<String>,
        dry_run: bool,
        json: bool,
        recursive: bool,
        filters: Option<Vec<String>>,
        provenance: bool,
    },
    /// List staged versions.
    List { package: Option<String>, json: bool },
    /// Show details about a staged version.
    View { stage_id: String, json: bool },
    /// Download the staged tarball for inspection.
    Download { stage_id: String },
    /// Promote a staged version to the live registry (2FA required).
    Approve { stage_id: String, otp: Option<String> },
    /// Discard a staged version (2FA required).
    Reject { stage_id: String, otp: Option<String> },
}

/// Options for the stage command.
#[derive(Debug)]
pub struct StageCommandOptions<'a> {
    pub subcommand: StageSubcommand,
    pub registry: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the stage command with the package manager.
    #[must_use]
    pub async fn run_stage_command(
        &self,
        options: &StageCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_stage_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the stage command.
    ///
    /// pnpm and npm pass through directly. yarn berry uses its npm plugin
    /// (`yarn npm publish --staged` to stage, `yarn npm stage …` to manage),
    /// falling back to npm for `view`/`download` which yarn does not expose.
    /// yarn 1 and bun have no staged-publishing support and fall back to npm.
    ///
    /// Note: `yarn stage` is git/VCS staging, not publishing — it is never used
    /// here.
    #[must_use]
    pub fn resolve_stage_command(&self, options: &StageCommandOptions) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        let bin_name: String;

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();

                // pnpm: --filter must come before the command. `--filter` and
                // `--recursive` are pnpm-publish-only workspace flags, so both
                // live here rather than in the shared subcommand builder.
                if let StageSubcommand::Publish { filters: Some(filters), .. } = &options.subcommand
                {
                    for filter in filters {
                        args.push("--filter".into());
                        args.push(filter.clone());
                    }
                }

                args.push("stage".into());
                append_stage_subcommand(&mut args, &options.subcommand);

                if let StageSubcommand::Publish { recursive: true, .. } = &options.subcommand {
                    args.push("--recursive".into());
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                append_npm_stage(&mut args, &options.subcommand);
            }
            PackageManagerType::Yarn => {
                if self.is_yarn_berry() {
                    match &options.subcommand {
                        StageSubcommand::Publish { target: Some(_), .. } => {
                            // `yarn npm publish` has no target argument; it always
                            // stages the active workspace. To honor an explicit
                            // tarball/folder, stage it through npm instead (npm
                            // staged publishing accepts a target), matching how
                            // `vp pm publish` delegates yarn -> npm.
                            output::warn(
                                "yarn cannot stage a prebuilt tarball or folder; using npm stage publish for the given target",
                            );
                            bin_name = "npm".into();
                            append_npm_stage(&mut args, &options.subcommand);
                        }
                        StageSubcommand::Publish { .. } => {
                            // yarn berry stages the workspace via `yarn npm publish --staged`.
                            bin_name = "yarn".into();
                            append_yarn_publish_staged(&mut args, &options.subcommand);
                        }
                        StageSubcommand::List { .. }
                        | StageSubcommand::Approve { .. }
                        | StageSubcommand::Reject { .. } => {
                            bin_name = "yarn".into();
                            args.push("npm".into());
                            args.push("stage".into());
                            append_stage_subcommand(&mut args, &options.subcommand);
                        }
                        StageSubcommand::View { .. } | StageSubcommand::Download { .. } => {
                            output::warn(
                                "yarn does not support 'stage view'/'stage download', falling back to npm stage",
                            );
                            bin_name = "npm".into();
                            append_npm_stage(&mut args, &options.subcommand);
                        }
                    }
                } else {
                    output::warn(
                        "yarn 1 does not support staged publishing, falling back to npm stage",
                    );
                    bin_name = "npm".into();
                    append_npm_stage(&mut args, &options.subcommand);
                }
            }
            PackageManagerType::Bun => {
                output::warn("bun does not support staged publishing, falling back to npm stage");
                bin_name = "npm".into();
                append_npm_stage(&mut args, &options.subcommand);
            }
        }

        // `--registry` is forwarded to npm/pnpm, which accept it. yarn's npm
        // plugin (`yarn npm publish`/`yarn npm stage`) does not take a
        // `--registry` flag — it resolves the registry from `.yarnrc.yml` — so
        // forwarding it would make yarn abort with an unknown-option error.
        if let Some(registry) = options.registry {
            if bin_name == "yarn" {
                output::warn(
                    "--registry is not supported by yarn's npm plugin (set the registry in .yarnrc.yml), ignoring flag",
                );
            } else {
                args.push("--registry".into());
                args.push(registry.to_string());
            }
        }

        // Add pass-through args.
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}

/// Build the `npm stage …` argument list (also used as the fallback path for
/// yarn 1, bun, and yarn berry's unsupported `view`/`download`).
fn append_npm_stage(args: &mut Vec<String>, subcommand: &StageSubcommand) {
    warn_npm_workspace_unsupported(subcommand);
    args.push("stage".into());
    append_stage_subcommand(args, subcommand);
}

/// Append the `<subcommand> [args]` portion shared by the npm/pnpm `stage` and
/// yarn `npm stage` paths. The pnpm-publish-only workspace flags
/// (`--filter`/`--recursive`) are emitted by the pnpm caller, not here.
fn append_stage_subcommand(args: &mut Vec<String>, subcommand: &StageSubcommand) {
    match subcommand {
        StageSubcommand::Publish {
            target,
            tag,
            access,
            otp,
            dry_run,
            json,
            provenance,
            recursive: _,
            filters: _,
        } => {
            args.push("publish".into());
            if let Some(target) = target {
                args.push(target.clone());
            }
            push_publish_flags(args, tag, access, otp, *dry_run, *json, *provenance);
        }
        StageSubcommand::List { package, json } => {
            args.push("list".into());
            if let Some(package) = package {
                args.push(package.clone());
            }
            if *json {
                args.push("--json".into());
            }
        }
        StageSubcommand::View { stage_id, json } => {
            args.push("view".into());
            args.push(stage_id.clone());
            if *json {
                args.push("--json".into());
            }
        }
        StageSubcommand::Download { stage_id } => {
            args.push("download".into());
            args.push(stage_id.clone());
        }
        StageSubcommand::Approve { stage_id, otp } => {
            args.push("approve".into());
            args.push(stage_id.clone());
            if let Some(otp) = otp {
                args.push("--otp".into());
                args.push(otp.clone());
            }
        }
        StageSubcommand::Reject { stage_id, otp } => {
            args.push("reject".into());
            args.push(stage_id.clone());
            if let Some(otp) = otp {
                args.push("--otp".into());
                args.push(otp.clone());
            }
        }
    }
}

/// Build `yarn npm publish --staged …`. yarn berry's npm plugin stages via the
/// publish command; `--tag`/`--access`/`--otp`/`--provenance`/`--dry-run`/`--json`
/// are all forwarded (yarn supports them). The `target` positional is handled by
/// the caller (routed to npm) and never reaches here, and `--recursive`/`--filter`
/// have no `yarn npm publish` equivalent so they are warned and dropped.
fn append_yarn_publish_staged(args: &mut Vec<String>, subcommand: &StageSubcommand) {
    let StageSubcommand::Publish {
        tag,
        access,
        otp,
        dry_run,
        json,
        recursive,
        filters,
        target: _,
        provenance,
    } = subcommand
    else {
        return;
    };

    args.push("npm".into());
    args.push("publish".into());
    args.push("--staged".into());
    push_publish_flags(args, tag, access, otp, *dry_run, *json, *provenance);

    if *recursive {
        output::warn("--recursive is not supported by yarn npm publish, ignoring flag");
    }
    if filters.as_ref().is_some_and(|filters| !filters.is_empty()) {
        output::warn("--filter is not supported by yarn npm publish, ignoring flag");
    }
}

/// Forward the publish flags common to the npm/pnpm `stage publish` path and the
/// yarn `npm publish --staged` path. Flag order is not significant to any of the
/// package managers, so a single canonical order is used.
fn push_publish_flags(
    args: &mut Vec<String>,
    tag: &Option<String>,
    access: &Option<String>,
    otp: &Option<String>,
    dry_run: bool,
    json: bool,
    provenance: bool,
) {
    if let Some(tag) = tag {
        args.push("--tag".into());
        args.push(tag.clone());
    }
    if let Some(access) = access {
        args.push("--access".into());
        args.push(access.clone());
    }
    if let Some(otp) = otp {
        args.push("--otp".into());
        args.push(otp.clone());
    }
    if dry_run {
        args.push("--dry-run".into());
    }
    if json {
        args.push("--json".into());
    }
    if provenance {
        args.push("--provenance".into());
    }
}

/// Warn about the workspace flags (`--recursive`/`--filter`) that npm staged
/// publishing cannot honor (only `publish` carries them).
fn warn_npm_workspace_unsupported(subcommand: &StageSubcommand) {
    if let StageSubcommand::Publish { recursive, filters, .. } = subcommand {
        if *recursive {
            output::warn("--recursive is not supported by npm staged publishing, ignoring flag");
        }
        if filters.as_ref().is_some_and(|filters| !filters.is_empty()) {
            output::warn("--filter is not supported by npm staged publishing, ignoring flag");
        }
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

    fn publish_sub_full(
        tag: Option<&str>,
        access: Option<&str>,
        recursive: bool,
        filters: Option<Vec<String>>,
        provenance: bool,
    ) -> StageSubcommand {
        StageSubcommand::Publish {
            target: None,
            tag: tag.map(Into::into),
            access: access.map(Into::into),
            otp: None,
            dry_run: false,
            json: false,
            recursive,
            filters,
            provenance,
        }
    }

    fn publish_sub() -> StageSubcommand {
        publish_sub_full(None, None, false, None, false)
    }

    fn opts(subcommand: StageSubcommand) -> StageCommandOptions<'static> {
        StageCommandOptions { subcommand, registry: None, pass_through_args: None }
    }

    #[test]
    fn test_pnpm_stage_publish() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.3.0");
        let result = pm.resolve_stage_command(&opts(publish_sub()));
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["stage", "publish"]);
    }

    #[test]
    fn test_pnpm_stage_publish_with_tag_access() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.3.0");
        let result = pm.resolve_stage_command(&opts(publish_sub_full(
            Some("next"),
            Some("public"),
            false,
            None,
            false,
        )));
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["stage", "publish", "--tag", "next", "--access", "public"]);
    }

    #[test]
    fn test_pnpm_stage_publish_recursive_filter() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.3.0");
        let result = pm.resolve_stage_command(&opts(publish_sub_full(
            None,
            None,
            true,
            Some(vec!["app".into()]),
            false,
        )));
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["--filter", "app", "stage", "publish", "--recursive"]);
    }

    #[test]
    fn test_npm_stage_publish() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.15.0");
        let result = pm.resolve_stage_command(&opts(publish_sub()));
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "publish"]);
    }

    #[test]
    fn test_npm_stage_publish_recursive_ignored() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.15.0");
        let result = pm.resolve_stage_command(&opts(publish_sub_full(
            None,
            None,
            true,
            Some(vec!["app".into()]),
            false,
        )));
        // npm staged publishing has no workspace flags; they are dropped.
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "publish"]);
    }

    #[test]
    fn test_npm_stage_list_with_package_json() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.15.0");
        let result = pm.resolve_stage_command(&opts(StageSubcommand::List {
            package: Some("my-pkg".into()),
            json: true,
        }));
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "list", "my-pkg", "--json"]);
    }

    #[test]
    fn test_npm_stage_view() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.15.0");
        let result = pm.resolve_stage_command(&opts(StageSubcommand::View {
            stage_id: "abc123".into(),
            json: false,
        }));
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "view", "abc123"]);
    }

    #[test]
    fn test_npm_stage_download() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.15.0");
        let result = pm
            .resolve_stage_command(&opts(StageSubcommand::Download { stage_id: "abc123".into() }));
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "download", "abc123"]);
    }

    #[test]
    fn test_stage_approve_with_otp() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.3.0");
        let result = pm.resolve_stage_command(&opts(StageSubcommand::Approve {
            stage_id: "abc123".into(),
            otp: Some("123456".into()),
        }));
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["stage", "approve", "abc123", "--otp", "123456"]);
    }

    #[test]
    fn test_stage_reject() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.15.0");
        let result = pm.resolve_stage_command(&opts(StageSubcommand::Reject {
            stage_id: "abc123".into(),
            otp: None,
        }));
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "reject", "abc123"]);
    }

    #[test]
    fn test_yarn_berry_stage_publish_uses_npm_plugin() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_stage_command(&opts(publish_sub_full(
            Some("next"),
            None,
            false,
            None,
            false,
        )));
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "publish", "--staged", "--tag", "next"]);
    }

    #[test]
    fn test_yarn_berry_stage_publish_forwards_dry_run_json_provenance() {
        // `yarn npm publish` supports --dry-run, --json, and --provenance, so
        // they must be forwarded (not warned-and-dropped).
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_stage_command(&opts(StageSubcommand::Publish {
            target: None,
            tag: None,
            access: None,
            otp: None,
            dry_run: true,
            json: true,
            recursive: false,
            filters: None,
            provenance: true,
        }));
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(
            result.args,
            vec!["npm", "publish", "--staged", "--dry-run", "--json", "--provenance"]
        );
    }

    #[test]
    fn test_yarn_berry_stage_publish_with_target_falls_back_to_npm() {
        // `yarn npm publish` has no target argument; honor the explicit tarball
        // via npm instead of silently staging the workspace package.
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_stage_command(&opts(StageSubcommand::Publish {
            target: Some("./pkg.tgz".into()),
            tag: None,
            access: None,
            otp: None,
            dry_run: false,
            json: false,
            recursive: false,
            filters: None,
            provenance: false,
        }));
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "publish", "./pkg.tgz"]);
    }

    #[test]
    fn test_yarn_berry_stage_registry_dropped() {
        // yarn's npm plugin does not accept --registry; it must be dropped (the
        // resolver warns) rather than forwarded into a yarn command that errors.
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_stage_command(&StageCommandOptions {
            subcommand: StageSubcommand::List { package: None, json: false },
            registry: Some("https://registry.example.com"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "stage", "list"]);
    }

    #[test]
    fn test_yarn_berry_stage_list() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result =
            pm.resolve_stage_command(&opts(StageSubcommand::List { package: None, json: false }));
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "stage", "list"]);
    }

    #[test]
    fn test_yarn_berry_stage_approve() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_stage_command(&opts(StageSubcommand::Approve {
            stage_id: "abc123".into(),
            otp: None,
        }));
        assert_eq!(result.bin_path, "yarn");
        assert_eq!(result.args, vec!["npm", "stage", "approve", "abc123"]);
    }

    #[test]
    fn test_yarn_berry_stage_view_falls_back_to_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm.resolve_stage_command(&opts(StageSubcommand::View {
            stage_id: "abc123".into(),
            json: false,
        }));
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "view", "abc123"]);
    }

    #[test]
    fn test_yarn1_stage_falls_back_to_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.0");
        let result = pm.resolve_stage_command(&opts(publish_sub()));
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "publish"]);
    }

    #[test]
    fn test_bun_stage_falls_back_to_npm() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.2.0");
        let result = pm.resolve_stage_command(&opts(publish_sub()));
        assert_eq!(result.bin_path, "npm");
        assert_eq!(result.args, vec!["stage", "publish"]);
    }

    #[test]
    fn test_stage_registry_appended() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.3.0");
        let result = pm.resolve_stage_command(&StageCommandOptions {
            subcommand: StageSubcommand::List { package: None, json: false },
            registry: Some("https://registry.example.com"),
            pass_through_args: None,
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(
            result.args,
            vec!["stage", "list", "--registry", "https://registry.example.com"]
        );
    }

    #[test]
    fn test_stage_pass_through_args() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.3.0");
        let extra = vec!["--foo".to_string()];
        let result = pm.resolve_stage_command(&StageCommandOptions {
            subcommand: publish_sub(),
            registry: None,
            pass_through_args: Some(&extra),
        });
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["stage", "publish", "--foo"]);
    }
}
