use vp_pm_cli_macros::pm_args;

use crate::resolution::{
    Bun, CommandBuilder, CommandResolution, DiagnosticKind, Diagnostics, Npm, Pnpm, Resolve, Yarn,
};

/// Staged-publishing subcommands (`vp pm stage <subcommand>`).
///
/// Maps to `npm stage`/`pnpm stage` and Yarn's npm plugin starting with 4.16.0
/// (`yarn npm publish --staged`, `yarn npm stage ...`). Note: this is unrelated
/// to yarn's own `yarn stage` command, which stages files for a VCS commit.
#[pm_args]
#[derive(clap::Subcommand, Clone, Debug, PartialEq, Eq)]
pub enum StageCommand {
    /// Stage a package for publishing (no 2FA required)
    Publish {
        /// Tarball or folder to stage
        #[arg(value_name = "TARBALL|FOLDER")]
        target: Option<String>,

        /// Publish tag
        #[arg(long)]
        tag: Option<String>,

        /// Access level (public/restricted)
        #[arg(long)]
        access: Option<String>,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Preview without staging
        #[arg(long)]
        dry_run: bool,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Stage all publishable workspace packages
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Filter packages in monorepo
        #[arg(long, value_name = "PATTERN")]
        filter: Option<Vec<String>>,

        /// Stage with provenance
        #[arg(long)]
        provenance: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// List staged versions
    #[command(visible_alias = "ls")]
    List {
        /// Package spec to filter by
        package: Option<String>,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL", not_supported(yarn >= "4.16.0"))]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Show details about a staged version
    View {
        /// Stage ID
        stage_id: String,

        /// Output in JSON format
        #[arg(long)]
        json: bool,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Download the staged tarball for inspection
    Download {
        /// Stage ID
        stage_id: String,

        /// Registry URL
        #[arg(long, value_name = "URL")]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Promote a staged version to the live registry (2FA required)
    Approve {
        /// Stage ID
        stage_id: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Registry URL
        #[arg(long, value_name = "URL", not_supported(yarn >= "4.16.0"))]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },

    /// Discard a staged version (2FA required)
    Reject {
        /// Stage ID
        stage_id: String,

        /// One-time password for authentication
        #[arg(long, value_name = "OTP")]
        otp: Option<String>,

        /// Registry URL
        #[arg(long, value_name = "URL", not_supported(yarn >= "4.16.0"))]
        registry: Option<String>,

        /// Additional arguments
        #[arg(last = true, allow_hyphen_values = true)]
        pass_through_args: Vec<String>,
    },
}

impl Resolve<StageCommand> for Pnpm {
    fn resolve(&self, args: &StageCommand, _diag: &mut Diagnostics) -> CommandResolution {
        let mut cmd = CommandBuilder::new("pnpm");
        if let StageCommand::Publish { filter: Some(filters), .. } = args {
            cmd.repeated("--filter", filters.iter());
        }
        cmd.arg("stage");
        append_stage_subcommand(&mut cmd, args);
        if let StageCommand::Publish { recursive: true, .. } = args {
            cmd.arg("--recursive");
        }
        append_registry_and_pass_through(&mut cmd, args);
        cmd.into()
    }
}

impl Npm {
    fn resolve_stage(args: &StageCommand) -> CommandResolution {
        let mut cmd = CommandBuilder::new("npm");
        cmd.arg("stage");
        append_stage_subcommand(&mut cmd, args);
        if let StageCommand::Publish { target, recursive, filter, .. } = args {
            // npm ignores workspace selection for explicit targets other than '.'.
            // https://github.com/npm/cli/blob/b888cc9a9ff34a8b023ff47b784692396635397b/lib/commands/publish.js#L54-L60
            if target.as_ref().is_some_and(|target| target != ".")
                && (*recursive || filter.as_ref().is_some_and(|filters| !filters.is_empty()))
            {
                return CommandResolution::InvalidArgument(
                    "npm staged publishing cannot combine --recursive or --filter with an explicit tarball or folder other than '.'.".into(),
                );
            }
            cmd.arg_if("--workspaces", *recursive);
            if let Some(filters) = filter {
                cmd.repeated("--workspace", filters.iter());
            }
        }
        append_registry_and_pass_through(&mut cmd, args);
        cmd.into()
    }
}

impl Resolve<StageCommand> for Npm {
    fn resolve(&self, args: &StageCommand, _diag: &mut Diagnostics) -> CommandResolution {
        Self::resolve_stage(args)
    }
}

impl Resolve<StageCommand> for Yarn {
    fn resolve(&self, args: &StageCommand, diag: &mut Diagnostics) -> CommandResolution {
        if !self.supports_v4_16_commands() {
            diag.warn(
                DiagnosticKind::FallbackCommand,
                "yarn < 4.16.0 does not support staged publishing, falling back to npm stage",
            );
            return Npm::resolve_stage(args);
        }

        let mut cmd = match args {
            StageCommand::Publish { target: Some(_), .. } => {
                diag.warn(
                DiagnosticKind::FallbackCommand,
                "yarn cannot stage a prebuilt tarball or folder; using npm stage publish for the given target",
            );
                return Npm::resolve_stage(args);
            }
            StageCommand::Publish { recursive, filter, registry, .. } => {
                let mut errors = Vec::new();
                if *recursive || filter.as_ref().is_some_and(|filters| !filters.is_empty()) {
                    errors.push(
                        "yarn does not support --recursive or --filter for native staged publishing.",
                    );
                }
                // `yarn npm publish` has no --registry; use npmPublishRegistry instead.
                // https://yarnpkg.com/cli/npm/publish
                if registry.is_some() {
                    errors.push("yarn >= 4.16.0 does not support --registry.");
                }
                if !errors.is_empty() {
                    return CommandResolution::InvalidArgument(errors.join("\n"));
                }
                let mut cmd = CommandBuilder::new("yarn");
                append_yarn_publish_staged(&mut cmd, args);
                cmd
            }
            StageCommand::List { .. }
            | StageCommand::Approve { .. }
            | StageCommand::Reject { .. } => {
                let mut cmd = CommandBuilder::new("yarn");
                cmd.arg("npm").arg("stage");
                append_stage_subcommand(&mut cmd, args);
                cmd
            }
            StageCommand::View { .. } | StageCommand::Download { .. } => {
                diag.warn(
                DiagnosticKind::FallbackCommand,
                "yarn does not support 'stage view'/'stage download', falling back to npm stage",
            );
                return Npm::resolve_stage(args);
            }
        };
        append_registry_and_pass_through(&mut cmd, args);
        cmd.into()
    }
}

impl Resolve<StageCommand> for Bun {
    fn resolve(&self, args: &StageCommand, diag: &mut Diagnostics) -> CommandResolution {
        diag.warn(
            DiagnosticKind::FallbackCommand,
            "bun does not support staged publishing, falling back to npm stage",
        );
        Npm::resolve_stage(args)
    }
}

fn append_stage_subcommand(cmd: &mut CommandBuilder, command: &StageCommand) {
    match command {
        StageCommand::Publish {
            target,
            tag,
            access,
            otp,
            dry_run,
            json,
            recursive: _,
            filter: _,
            provenance,
            registry: _,
            pass_through_args: _,
        } => {
            cmd.arg("publish");
            if let Some(target) = target {
                cmd.arg(target);
            }
            push_publish_flags(cmd, tag, access, otp, *dry_run, *json, *provenance);
        }
        StageCommand::List { package, json, .. } => {
            cmd.arg("list");
            if let Some(package) = package {
                cmd.arg(package);
            }
            cmd.arg_if("--json", *json);
        }
        StageCommand::View { stage_id, json, .. } => {
            cmd.arg("view").arg(stage_id).arg_if("--json", *json);
        }
        StageCommand::Download { stage_id, .. } => {
            cmd.arg("download").arg(stage_id);
        }
        StageCommand::Approve { stage_id, otp, .. } => {
            cmd.arg("approve").arg(stage_id).option("--otp", otp.as_ref());
        }
        StageCommand::Reject { stage_id, otp, .. } => {
            cmd.arg("reject").arg(stage_id).option("--otp", otp.as_ref());
        }
    }
}

fn append_yarn_publish_staged(cmd: &mut CommandBuilder, command: &StageCommand) {
    let StageCommand::Publish { tag, access, otp, dry_run, json, provenance, .. } = command else {
        return;
    };

    cmd.arg("npm").arg("publish").arg("--staged");
    push_publish_flags(cmd, tag, access, otp, *dry_run, *json, *provenance);
}

fn push_publish_flags(
    cmd: &mut CommandBuilder,
    tag: &Option<String>,
    access: &Option<String>,
    otp: &Option<String>,
    dry_run: bool,
    json: bool,
    provenance: bool,
) {
    cmd.option("--tag", tag.as_ref())
        .option("--access", access.as_ref())
        .option("--otp", otp.as_ref())
        .arg_if("--dry-run", dry_run)
        .arg_if("--json", json)
        .arg_if("--provenance", provenance);
}

fn append_registry_and_pass_through(cmd: &mut CommandBuilder, command: &StageCommand) {
    cmd.option("--registry", command.registry()).extend(command.pass_through_args().iter());
}

impl StageCommand {
    fn registry(&self) -> Option<&String> {
        match self {
            Self::Publish { registry, .. }
            | Self::List { registry, .. }
            | Self::View { registry, .. }
            | Self::Download { registry, .. }
            | Self::Approve { registry, .. }
            | Self::Reject { registry, .. } => registry.as_ref(),
        }
    }

    fn pass_through_args(&self) -> &[String] {
        match self {
            Self::Publish { pass_through_args, .. }
            | Self::List { pass_through_args, .. }
            | Self::View { pass_through_args, .. }
            | Self::Download { pass_through_args, .. }
            | Self::Approve { pass_through_args, .. }
            | Self::Reject { pass_through_args, .. } => pass_through_args,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::{
        Resolution, resolve,
        test_utils::{bun, expect_run, expect_unsupported, npm, parse_subcommand, pnpm, yarn},
    };

    fn publish_sub_full(
        tag: Option<&str>,
        access: Option<&str>,
        recursive: bool,
        filter: Option<Vec<String>>,
        provenance: bool,
    ) -> StageCommand {
        StageCommand::Publish {
            target: None,
            tag: tag.map(Into::into),
            access: access.map(Into::into),
            otp: None,
            dry_run: false,
            json: false,
            recursive,
            filter,
            provenance,
            registry: None,
            pass_through_args: Vec::new(),
        }
    }

    fn publish_sub() -> StageCommand {
        publish_sub_full(None, None, false, None, false)
    }

    #[test]
    fn test_parser_accepts_publish_subcommand() {
        let args = parse_subcommand::<StageCommand>([
            "publish",
            "./pkg.tgz",
            "--tag",
            "next",
            "--filter",
            "app",
            "--",
            "--foo",
        ])
        .unwrap();

        assert_eq!(
            args,
            StageCommand::Publish {
                target: Some("./pkg.tgz".to_string()),
                tag: Some("next".to_string()),
                access: None,
                otp: None,
                dry_run: false,
                json: false,
                recursive: false,
                filter: Some(vec!["app".to_string()]),
                provenance: false,
                registry: None,
                pass_through_args: vec!["--foo".to_string()],
            }
        );
    }

    #[test]
    fn test_pnpm_stage_publish() {
        let command = expect_run(resolve(&pnpm("11.3.0"), publish_sub()).outcome);

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["stage", "publish"]);
    }

    #[test]
    fn test_pnpm_stage_publish_with_tag_access() {
        let command = expect_run(
            resolve(
                &pnpm("11.3.0"),
                publish_sub_full(Some("next"), Some("public"), false, None, false),
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["stage", "publish", "--tag", "next", "--access", "public"]);
    }

    #[test]
    fn test_pnpm_stage_publish_recursive_filter() {
        let command = expect_run(
            resolve(
                &pnpm("11.3.0"),
                publish_sub_full(None, None, true, Some(vec!["app".into()]), false),
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["--filter", "app", "stage", "publish", "--recursive"]);
    }

    #[test]
    fn test_npm_stage_publish() {
        let command = expect_run(resolve(&npm("11.15.0"), publish_sub()).outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["stage", "publish"]);
    }

    #[test]
    fn test_npm_stage_publish_workspace_selection() {
        for version in ["11.16.0", "12.0.2"] {
            for (args, expected) in [
                (vec!["--recursive"], vec!["--workspaces"]),
                (vec!["--filter", "app"], vec!["--workspace", "app"]),
                (
                    vec!["--filter", "app", "--filter", "lib"],
                    vec!["--workspace", "app", "--workspace", "lib"],
                ),
                (
                    vec!["--recursive", "--filter", "app"],
                    vec!["--workspaces", "--workspace", "app"],
                ),
                (
                    vec![".", "--recursive", "--filter", "app"],
                    vec![".", "--workspaces", "--workspace", "app"],
                ),
            ] {
                let args = parse_subcommand::<StageCommand>(std::iter::once("publish").chain(args))
                    .unwrap();
                let Resolution { outcome, diagnostics } = resolve(&npm(version), args);
                let command = expect_run(outcome);
                assert_eq!(command.program, "npm");
                assert_eq!(command.args, [vec!["stage", "publish"], expected].concat());
                assert!(diagnostics.is_empty());
            }
        }
    }

    #[test]
    fn test_stage_npm_fallback_preserves_workspace_selection() {
        let args = parse_subcommand::<StageCommand>([
            "publish",
            ".",
            "--recursive",
            "--filter",
            "app",
            "--filter",
            "lib",
        ])
        .unwrap();
        for resolution in [
            resolve(&yarn("1.22.22"), args.clone()),
            resolve(&yarn("4.10.3"), args.clone()),
            resolve(&bun("1.4.0"), args),
        ] {
            let command = expect_run(resolution.outcome);
            assert_eq!(command.program, "npm");
            assert_eq!(
                command.args,
                vec![
                    "stage",
                    "publish",
                    ".",
                    "--workspaces",
                    "--workspace",
                    "app",
                    "--workspace",
                    "lib",
                ]
            );
            assert_eq!(resolution.diagnostics.len(), 1);
            assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::FallbackCommand);
        }
    }

    #[test]
    fn test_stage_npm_rejects_explicit_target_with_workspace_selection() {
        for target in ["./pkg.tgz", "./packages/app"] {
            for flags in [
                vec!["--recursive"],
                vec!["--filter", "app"],
                vec!["--recursive", "--filter", "app"],
            ] {
                let args =
                    parse_subcommand::<StageCommand>(["publish", target].into_iter().chain(flags))
                        .unwrap();
                for resolution in [
                    resolve(&npm("11.16.0"), args.clone()),
                    resolve(&npm("12.0.2"), args.clone()),
                    resolve(&yarn("1.22.22"), args.clone()),
                    resolve(&yarn("4.10.3"), args.clone()),
                    resolve(&bun("1.4.0"), args.clone()),
                ] {
                    assert_eq!(resolution.outcome, CommandResolution::InvalidArgument(
                        "npm staged publishing cannot combine --recursive or --filter with an explicit tarball or folder other than '.'.".into(),
                    ));
                }
            }
        }
    }

    #[test]
    fn test_npm_stage_publish_explicit_target_without_workspace_selection() {
        for target in ["./pkg.tgz", "./packages/app"] {
            let mut args = publish_sub();
            if let StageCommand::Publish { target: value, filter, .. } = &mut args {
                *value = Some(target.into());
                *filter = Some(Vec::new());
            }
            let Resolution { outcome, diagnostics } = resolve(&npm("11.16.0"), args);
            assert_eq!(expect_run(outcome).args, vec!["stage", "publish", target]);
            assert!(diagnostics.is_empty());
        }
    }

    #[test]
    fn test_npm_stage_publish_workspace_flags_registry_and_pass_through() {
        let args = parse_subcommand::<StageCommand>([
            "publish",
            "--recursive",
            "--filter",
            "app",
            "--dry-run",
            "--json",
            "--registry",
            "http://127.0.0.1:9",
            "--",
            "--ignore-scripts",
        ])
        .unwrap();
        let Resolution { outcome, diagnostics } = resolve(&npm("11.16.0"), args);
        assert_eq!(
            expect_run(outcome).args,
            vec![
                "stage",
                "publish",
                "--dry-run",
                "--json",
                "--workspaces",
                "--workspace",
                "app",
                "--registry",
                "http://127.0.0.1:9",
                "--ignore-scripts",
            ]
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_npm_stage_list_with_package_json() {
        let command = expect_run(
            resolve(
                &npm("11.15.0"),
                StageCommand::List {
                    package: Some("my-pkg".into()),
                    json: true,
                    registry: None,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["stage", "list", "my-pkg", "--json"]);
    }

    #[test]
    fn test_npm_stage_view() {
        let command = expect_run(
            resolve(
                &npm("11.15.0"),
                StageCommand::View {
                    stage_id: "abc123".into(),
                    json: false,
                    registry: None,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["stage", "view", "abc123"]);
    }

    #[test]
    fn test_npm_stage_download() {
        let command = expect_run(
            resolve(
                &npm("11.15.0"),
                StageCommand::Download {
                    stage_id: "abc123".into(),
                    registry: None,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["stage", "download", "abc123"]);
    }

    #[test]
    fn test_stage_approve_with_otp() {
        let command = expect_run(
            resolve(
                &pnpm("11.3.0"),
                StageCommand::Approve {
                    stage_id: "abc123".into(),
                    otp: Some("123456".into()),
                    registry: None,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["stage", "approve", "abc123", "--otp", "123456"]);
    }

    #[test]
    fn test_stage_reject() {
        let command = expect_run(
            resolve(
                &npm("11.15.0"),
                StageCommand::Reject {
                    stage_id: "abc123".into(),
                    otp: None,
                    registry: None,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["stage", "reject", "abc123"]);
    }

    #[test]
    fn test_yarn_berry_stage_publish_uses_npm_plugin() {
        let command = expect_run(
            resolve(&yarn("4.16.0"), publish_sub_full(Some("next"), None, false, None, false))
                .outcome,
        );

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "publish", "--staged", "--tag", "next"]);
    }

    #[test]
    fn test_yarn_berry_stage_publish_forwards_dry_run_json_provenance() {
        let command = expect_run(
            resolve(
                &yarn("4.16.0"),
                StageCommand::Publish {
                    target: None,
                    tag: None,
                    access: None,
                    otp: None,
                    dry_run: true,
                    json: true,
                    recursive: false,
                    filter: None,
                    provenance: true,
                    registry: None,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec!["npm", "publish", "--staged", "--dry-run", "--json", "--provenance"]
        );
    }

    #[test]
    fn test_yarn_berry_stage_publish_with_target_falls_back_to_npm() {
        let Resolution { outcome, diagnostics } = resolve(
            &yarn("4.16.0"),
            StageCommand::Publish {
                target: Some("./pkg.tgz".into()),
                tag: None,
                access: None,
                otp: None,
                dry_run: false,
                json: false,
                recursive: false,
                filter: None,
                provenance: false,
                registry: None,
                pass_through_args: Vec::new(),
            },
        );
        let command = expect_run(outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["stage", "publish", "./pkg.tgz"]);
        assert_eq!(
            diagnostics[0].message,
            "yarn cannot stage a prebuilt tarball or folder; using npm stage publish for the given target"
        );
    }

    #[test]
    fn test_yarn_native_stage_rejects_registry_on_management_commands() {
        for version in ["4.16.0", "4.18.0"] {
            for command in [vec!["list"], vec!["approve", "abc123"], vec!["reject", "abc123"]] {
                let args = parse_subcommand::<StageCommand>(command.into_iter().chain([
                    "--registry",
                    "https://registry.example.com",
                    "--",
                    "--help",
                ]))
                .unwrap();
                expect_unsupported(
                    resolve(&yarn(version), args),
                    &["yarn >= 4.16.0 does not support --registry."],
                );
            }
        }
    }

    #[test]
    fn test_yarn_native_stage_rejects_workspace_selection() {
        for version in ["4.16.0", "4.18.0"] {
            let args = parse_subcommand::<StageCommand>([
                "publish",
                "--recursive",
                "--filter",
                "app",
                "--registry",
                "https://registry.example.com",
                "--",
                "--help",
            ])
            .unwrap();
            expect_unsupported(
                resolve(&yarn(version), args),
                &[
                    "yarn does not support --recursive or --filter for native staged publishing.",
                    "yarn >= 4.16.0 does not support --registry.",
                ],
            );
        }
    }

    #[test]
    fn test_yarn_native_stage_publish_rejects_registry() {
        for version in ["4.16.0", "4.18.0"] {
            let args = parse_subcommand::<StageCommand>([
                "publish",
                "--dry-run",
                "--registry",
                "https://registry.example.com",
                "--",
                "--help",
            ])
            .unwrap();
            expect_unsupported(
                resolve(&yarn(version), args),
                &["yarn >= 4.16.0 does not support --registry."],
            );
        }
    }

    #[test]
    fn test_yarn_stage_before_4_16_falls_back_to_npm() {
        for version in ["1.22.22", "2.4.2", "3.6.0", "4.15.0", "4.16.0-rc.0"] {
            for command in [
                vec!["publish", "--recursive", "--filter", "app"],
                vec!["list", "my-pkg", "--json"],
                vec!["view", "abc123", "--json"],
                vec!["download", "abc123"],
                vec!["approve", "abc123", "--otp", "123456"],
                vec!["reject", "abc123", "--otp", "123456"],
            ] {
                let args = parse_subcommand::<StageCommand>(command.into_iter().chain([
                    "--registry",
                    "https://registry.example.com",
                    "--",
                    "--help",
                ]))
                .unwrap();
                let resolution = resolve(&yarn(version), args.clone());
                assert_eq!(resolution.outcome, resolve(&npm("11.16.0"), args).outcome);
                assert_eq!(resolution.diagnostics.len(), 1);
                assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::FallbackCommand);
            }
        }
    }

    #[test]
    fn test_yarn_native_stage_preserves_npm_fallback_options() {
        for version in ["4.16.0", "4.18.0"] {
            for command in [
                vec!["publish", ".", "--recursive", "--filter", "app"],
                vec!["publish", "./pkg.tgz"],
                vec!["view", "abc123", "--json"],
                vec!["download", "abc123"],
            ] {
                let args = parse_subcommand::<StageCommand>(command.into_iter().chain([
                    "--registry",
                    "https://registry.example.com",
                    "--",
                    "--help",
                ]))
                .unwrap();
                let resolution = resolve(&yarn(version), args.clone());
                assert_eq!(resolution.outcome, resolve(&npm("11.16.0"), args).outcome);
                assert_eq!(resolution.diagnostics.len(), 1);
                assert_eq!(resolution.diagnostics[0].kind, DiagnosticKind::FallbackCommand);
            }
        }
    }

    #[test]
    fn test_yarn_native_stage_preserves_raw_args_and_empty_filter() {
        let mut args = publish_sub_full(None, None, false, Some(Vec::new()), false);
        if let StageCommand::Publish { pass_through_args, .. } = &mut args {
            *pass_through_args = vec![
                "--recursive".into(),
                "--filter".into(),
                "app".into(),
                "--registry".into(),
                "https://registry.example.com".into(),
            ];
        }
        let resolution = resolve(&yarn("4.16.0"), args);
        let command = expect_run(resolution.outcome);
        assert_eq!(command.program, "yarn");
        assert_eq!(
            command.args,
            vec![
                "npm",
                "publish",
                "--staged",
                "--recursive",
                "--filter",
                "app",
                "--registry",
                "https://registry.example.com"
            ]
        );
        assert!(resolution.diagnostics.is_empty());
    }

    #[test]
    fn test_yarn_berry_stage_list() {
        let command = expect_run(
            resolve(
                &yarn("4.16.0"),
                StageCommand::List {
                    package: None,
                    json: false,
                    registry: None,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "stage", "list"]);
    }

    #[test]
    fn test_yarn_berry_stage_approve() {
        let command = expect_run(
            resolve(
                &yarn("4.16.0"),
                StageCommand::Approve {
                    stage_id: "abc123".into(),
                    otp: None,
                    registry: None,
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "yarn");
        assert_eq!(command.args, vec!["npm", "stage", "approve", "abc123"]);
    }

    #[test]
    fn test_yarn_berry_stage_view_falls_back_to_npm() {
        let Resolution { outcome, diagnostics } = resolve(
            &yarn("4.16.0"),
            StageCommand::View {
                stage_id: "abc123".into(),
                json: false,
                registry: None,
                pass_through_args: Vec::new(),
            },
        );
        let command = expect_run(outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["stage", "view", "abc123"]);
        assert_eq!(
            diagnostics[0].message,
            "yarn does not support 'stage view'/'stage download', falling back to npm stage"
        );
    }

    #[test]
    fn test_yarn1_stage_falls_back_to_npm() {
        let Resolution { outcome, diagnostics } = resolve(&yarn("1.22.0"), publish_sub());
        let command = expect_run(outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["stage", "publish"]);
        assert_eq!(
            diagnostics[0].message,
            "yarn < 4.16.0 does not support staged publishing, falling back to npm stage"
        );
    }

    #[test]
    fn test_bun_stage_falls_back_to_npm() {
        let Resolution { outcome, diagnostics } = resolve(&bun("1.2.0"), publish_sub());
        let command = expect_run(outcome);

        assert_eq!(command.program, "npm");
        assert_eq!(command.args, vec!["stage", "publish"]);
        assert_eq!(
            diagnostics[0].message,
            "bun does not support staged publishing, falling back to npm stage"
        );
    }

    #[test]
    fn test_stage_registry_appended() {
        let command = expect_run(
            resolve(
                &pnpm("11.3.0"),
                StageCommand::List {
                    package: None,
                    json: false,
                    registry: Some("https://registry.example.com".into()),
                    pass_through_args: Vec::new(),
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(
            command.args,
            vec!["stage", "list", "--registry", "https://registry.example.com"]
        );
    }

    #[test]
    fn test_stage_pass_through_args() {
        let command = expect_run(
            resolve(
                &pnpm("11.3.0"),
                StageCommand::Publish {
                    target: None,
                    tag: None,
                    access: None,
                    otp: None,
                    dry_run: false,
                    json: false,
                    recursive: false,
                    filter: None,
                    provenance: false,
                    registry: None,
                    pass_through_args: vec!["--foo".to_string()],
                },
            )
            .outcome,
        );

        assert_eq!(command.program, "pnpm");
        assert_eq!(command.args, vec!["stage", "publish", "--foo"]);
    }
}
