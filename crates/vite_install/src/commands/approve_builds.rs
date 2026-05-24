use std::{collections::HashMap, process::ExitStatus};

use node_semver::{Range, Version};
use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env,
};

/// Options for the approve-builds command.
#[derive(Debug, Default)]
pub struct ApproveBuildsCommandOptions<'a> {
    /// Packages to approve. Prefix with `!` to deny (pnpm only).
    pub packages: &'a [String],
    /// Approve every package that is currently pending approval.
    pub all: bool,
    /// Extra arguments forwarded verbatim to the underlying package manager.
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the approve-builds command with the package manager.
    /// Returns `ExitStatus` with success (0) when the command is a no-op
    /// (npm, yarn, or bun with only deny tokens / no positionals).
    pub async fn run_approve_builds_command(
        &self,
        options: &ApproveBuildsCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let Some(resolved) = self.resolve_approve_builds_command(options)? else {
            return Ok(ExitStatus::default());
        };
        run_command(&resolved.bin_path, &resolved.args, &resolved.envs, cwd).await
    }

    /// Resolve the approve-builds command.
    /// Returns `None` when the command is a no-op for the detected PM
    /// (npm/yarn warn; bun with no approve-tokens prints a contextual hint).
    /// Returns `Err(Error::InvalidArgument)` when `--all` or `!pkg` is requested
    /// on a pnpm version that does not support it.
    pub fn resolve_approve_builds_command(
        &self,
        options: &ApproveBuildsCommandOptions,
    ) -> Result<Option<ResolveCommandResult>, Error> {
        // Up-front guard: `--all` is incompatible with positional package names
        // on both pnpm (ERR_PNPM_APPROVE_BUILDS_ALL_WITH_ARGS) and bun. clap's
        // `conflicts_with` catches direct positionals, but tokens passed via
        // `--` end up in `pass_through_args` and slip past — catch them here.
        if options.all
            && options.pass_through_args.is_some_and(|extras| extras.iter().any(is_positional_arg))
        {
            return Err(Error::InvalidArgument(
                "`--all` cannot be combined with positional package names (including via `--`)."
                    .into(),
            ));
        }

        let bin_name: String;
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                if options.all && !pnpm_supports_approve_builds_all(&self.version) {
                    return Err(Error::InvalidArgument(
                        "`--all` requires pnpm >= 10.32.0. Upgrade pnpm or pass package names explicitly.".into(),
                    ));
                }
                if options.packages.iter().any(|p| p.starts_with('!'))
                    && !pnpm_supports_deny_syntax(&self.version)
                {
                    return Err(Error::InvalidArgument(
                        "`!<pkg>` deny syntax requires pnpm >= 11.0.0. Upgrade pnpm or omit the `!` entries.".into(),
                    ));
                }
                bin_name = "pnpm".into();
                args.push("approve-builds".into());
                if options.all {
                    args.push("--all".into());
                }
                args.extend(options.packages.iter().cloned());
            }
            PackageManagerType::Bun => {
                // bun has no allow/deny model — filter `!pkg` with a warning.
                let (denies, approves): (Vec<&String>, Vec<&String>) =
                    options.packages.iter().partition(|p| p.starts_with('!'));
                let has_denies = !denies.is_empty();
                if has_denies {
                    let names: Vec<&str> =
                        denies.iter().map(|p| p.strip_prefix('!').unwrap_or(p)).collect();
                    output::warn(&format!(
                        "bun does not support denylisting build scripts. Packages outside \
                         `trustedDependencies` in package.json are already denied by default. \
                         Skipping: {}",
                        names.join(", ")
                    ));
                }

                // No approves and not --all: bun has no interactive picker.
                if approves.is_empty() && !options.all {
                    // If we already warned about denies, that message is enough context.
                    if !has_denies {
                        output::note(
                            "bun pm trust requires package names. Run `bun pm untrusted` to see \
                             which packages are pending, then pass them explicitly: \
                             `vp pm approve-builds <pkg> [<pkg>...]` or `vp pm approve-builds --all`.",
                        );
                    }
                    warn_dropped_pass_through(options.pass_through_args);
                    return Ok(None);
                }

                bin_name = "bun".into();
                args.push("pm".into());
                args.push("trust".into());
                if options.all {
                    args.push("--all".into());
                }
                args.extend(approves.into_iter().cloned());
            }
            PackageManagerType::Npm => {
                output::warn(
                    "npm runs lifecycle scripts by default. To restrict them, set \
                     `ignore-scripts=true` in .npmrc and rebuild approved packages with \
                     `vp pm rebuild <package>`.",
                );
                warn_dropped_pass_through(options.pass_through_args);
                return Ok(None);
            }
            PackageManagerType::Yarn => {
                // Yarn 1 (Classic) runs lifecycle scripts by default; Berry (2+) blocks them.
                if self.version.starts_with("1.") {
                    output::warn(
                        "yarn (v1) runs lifecycle scripts by default. To restrict them, set \
                         `ignore-scripts=true` in .npmrc and rebuild approved packages with \
                         `vp pm rebuild <package>`.",
                    );
                } else {
                    output::warn(
                        "yarn does not run third-party build scripts by default. To allow a \
                         package, set `dependenciesMeta[\"<package>\"].built: true` in package.json.",
                    );
                }
                warn_dropped_pass_through(options.pass_through_args);
                return Ok(None);
            }
        }

        // Append pass-through args to the underlying PM (pnpm/bun branches only).
        if let Some(extra) = options.pass_through_args {
            args.extend_from_slice(extra);
        }

        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        Ok(Some(ResolveCommandResult { bin_path: bin_name, args, envs }))
    }
}

fn pnpm_supports_approve_builds_all(version: &str) -> bool {
    // `pnpm approve-builds --all` was added in pnpm v10.32.0.
    // Uses npm-flavored range semantics (per node-semver): prereleases of
    // any version do NOT satisfy `>=10.32.0` by default. Users on a pnpm
    // prerelease should bump to the corresponding release before relying
    // on `--all`.
    version_satisfies(version, ">=10.32.0")
}

fn pnpm_supports_deny_syntax(version: &str) -> bool {
    // `!<pkg>` deny syntax shipped in pnpm v11.0.0 (PR #11030).
    // Same npm prerelease semantics as above.
    version_satisfies(version, ">=11.0.0")
}

fn version_satisfies(version: &str, range: &'static str) -> bool {
    // Static range strings always parse; unparsable user-supplied versions
    // are treated as not-satisfying (strict), since the production path
    // populates `version` from a validated semver.
    let range = range.parse::<Range>().expect("static range");
    version.parse::<Version>().is_ok_and(|v| v.satisfies(&range))
}

fn is_positional_arg(token: &String) -> bool {
    !token.starts_with('-')
}

fn warn_dropped_pass_through(extras: Option<&[String]>) {
    if let Some(extras) = extras
        && !extras.is_empty()
    {
        output::warn(&format!(
            "Ignoring pass-through args ({}): this package manager has no \
             native approve-builds command to forward them to.",
            extras.join(" ")
        ));
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
    fn pnpm_no_args_interactive() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.32.0");
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions::default())
            .expect("resolves")
            .expect("supported");
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["approve-builds"]);
    }

    #[test]
    fn pnpm_with_packages() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.32.0");
        let packages = vec!["esbuild".to_string(), "fsevents".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect("resolves")
            .expect("supported");
        assert_eq!(result.bin_path, "pnpm");
        assert_eq!(result.args, vec!["approve-builds", "esbuild", "fsevents"]);
    }

    #[test]
    fn pnpm_v11_passes_deny_syntax_through() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.0.0");
        let packages = vec!["esbuild".to_string(), "!core-js".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect("resolves")
            .expect("supported");
        assert_eq!(result.args, vec!["approve-builds", "esbuild", "!core-js"]);
    }

    #[test]
    fn pnpm_deny_rejected_below_v11() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.32.0");
        let packages = vec!["esbuild".to_string(), "!core-js".to_string()];
        let err = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect_err("should reject !pkg on pnpm < 11");
        assert!(matches!(err, Error::InvalidArgument(_)));
    }

    #[test]
    fn pnpm_deny_rejects_v11_prerelease() {
        // npm semver convention: prereleases don't satisfy `>=11.0.0`. Users
        // on `11.0.0-rc.0` must bump to a release before relying on `!pkg`.
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.0.0-rc.0");
        let packages = vec!["!core-js".to_string()];
        let err = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect_err("11.0.0-rc.0 should be rejected by deny gate (npm prerelease rule)");
        assert!(matches!(err, Error::InvalidArgument(_)));
    }

    #[test]
    fn pnpm_all_flag() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.32.0");
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            })
            .expect("resolves")
            .expect("supported");
        assert_eq!(result.args, vec!["approve-builds", "--all"]);
    }

    #[test]
    fn pnpm_all_flag_with_newer_version() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.0.0");
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            })
            .expect("resolves")
            .expect("supported");
        assert_eq!(result.args, vec!["approve-builds", "--all"]);
    }

    #[test]
    fn pnpm_all_rejected_below_v10_32() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.31.0");
        let err = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            })
            .expect_err("should reject --all on old pnpm");
        assert!(matches!(err, Error::InvalidArgument(_)));
    }

    #[test]
    fn pnpm_all_rejects_v10_32_prerelease() {
        // npm semver convention: prereleases of any version do NOT satisfy
        // `>=10.32.0`. Users on a 10.32.0-* tag must bump to 10.32.0 release.
        for version in &["10.32.0-0", "10.32.0-rc.0", "10.32.0-beta.1"] {
            let pm = create_mock_package_manager(PackageManagerType::Pnpm, version);
            let result = pm.resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            });
            assert!(
                matches!(result, Err(Error::InvalidArgument(_))),
                "{version}: expected rejection (npm prerelease rule), got {result:?}"
            );
        }
    }

    #[test]
    fn pnpm_all_rejects_v11_prerelease() {
        // Same npm prerelease rule: 11.0.0-rc.0 does not satisfy `>=10.32.0`.
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "11.0.0-rc.0");
        let err = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            })
            .expect_err("11.0.0-rc.0 should be rejected (npm prerelease rule)");
        assert!(matches!(err, Error::InvalidArgument(_)));
    }

    #[test]
    fn pnpm_all_rejected_at_v10_31_patch_max() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.31.999");
        let err = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            })
            .expect_err("10.31.999 should still reject");
        assert!(matches!(err, Error::InvalidArgument(_)));
    }

    #[test]
    fn pnpm_unparsable_version_rejects_all() {
        // Strict gate: unparsable versions (a corruption/edge case) fail the check.
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "latest");
        let err = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            })
            .expect_err("unparsable version should fail the gate");
        assert!(matches!(err, Error::InvalidArgument(_)));
    }

    #[test]
    fn pnpm_appends_pass_through_args() {
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.32.0");
        let extra = vec!["--workspace-root".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                pass_through_args: Some(&extra),
                ..Default::default()
            })
            .expect("resolves")
            .expect("supported");
        assert_eq!(result.args, vec!["approve-builds", "--all", "--workspace-root"]);
    }

    #[test]
    fn bun_trust_by_name() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.0");
        let packages = vec!["esbuild".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect("resolves")
            .expect("supported");
        assert_eq!(result.bin_path, "bun");
        assert_eq!(result.args, vec!["pm", "trust", "esbuild"]);
    }

    #[test]
    fn bun_trust_all() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.0");
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            })
            .expect("resolves")
            .expect("supported");
        assert_eq!(result.args, vec!["pm", "trust", "--all"]);
    }

    #[test]
    fn bun_filters_deny_syntax() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.0");
        let packages = vec!["esbuild".to_string(), "!core-js".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect("resolves")
            .expect("supported");
        // !core-js is filtered out, only esbuild forwarded
        assert_eq!(result.args, vec!["pm", "trust", "esbuild"]);
    }

    #[test]
    fn bun_only_deny_becomes_noop() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.0");
        let packages = vec!["!core-js".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect("resolves");
        // After filtering !core-js, no positionals remain → no-op.
        // The deny warn fired; no redundant no-args note.
        assert!(result.is_none());
    }

    #[test]
    fn bun_no_args_is_noop() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.0");
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions::default())
            .expect("resolves");
        assert!(result.is_none());
    }

    #[test]
    fn bun_strips_single_bang_only() {
        // `!!foo` strips exactly one leading `!` for the warning message,
        // preserving the user's intent in the displayed name.
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.0");
        let packages = vec!["!!core-js".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect("resolves");
        // Still classified as a deny (starts with !), but the displayed name
        // in the warning retains the second `!`. We can only assert no-op here;
        // visual inspection of the warn is captured in snap-tests.
        assert!(result.is_none());
    }

    #[test]
    fn bun_appends_pass_through_args() {
        let pm = create_mock_package_manager(PackageManagerType::Bun, "1.3.0");
        let extra = vec!["--silent".to_string()];
        let packages = vec!["esbuild".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                pass_through_args: Some(&extra),
                ..Default::default()
            })
            .expect("resolves")
            .expect("supported");
        assert_eq!(result.args, vec!["pm", "trust", "esbuild", "--silent"]);
    }

    #[test]
    fn npm_warns_and_noop() {
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let packages = vec!["esbuild".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect("resolves");
        assert!(result.is_none());
    }

    #[test]
    fn yarn_berry_warns_and_noop() {
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            })
            .expect("resolves");
        assert!(result.is_none());
    }

    #[test]
    fn yarn1_warns_and_noop() {
        // Yarn 1 emits an npm-style warning (lifecycle scripts run by default),
        // distinct from the Berry message about dependenciesMeta.built.
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.22");
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                ..Default::default()
            })
            .expect("resolves");
        assert!(result.is_none());
    }

    #[test]
    fn yarn1_prerelease_still_v1() {
        // A yarn 1.x prerelease (e.g. "1.22.22-canary") must still take the v1
        // branch, not the Berry branch. Guards against a future refactor that
        // strips prereleases via Version::parse + major-check.
        let pm = create_mock_package_manager(PackageManagerType::Yarn, "1.22.22-canary");
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions::default())
            .expect("resolves");
        assert!(result.is_none());
    }

    #[test]
    fn pnpm_deny_unparsable_version_rejects() {
        // Strict gate: the deny-syntax helper rejects unparsable versions for
        // the same reason as the `--all` gate (no semver, no support claim).
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "latest");
        let packages = vec!["!core-js".to_string()];
        let err = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                packages: &packages,
                ..Default::default()
            })
            .expect_err("unparsable version should fail the deny gate");
        assert!(matches!(err, Error::InvalidArgument(_)));
    }

    #[test]
    fn all_rejects_pass_through_positional() {
        // `--all` + a positional token slipped via `--` should be rejected
        // up-front (clap's conflicts_with can't catch the bypass).
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.32.0");
        let extras = vec!["esbuild".to_string()];
        let err = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                pass_through_args: Some(&extras),
                ..Default::default()
            })
            .expect_err("--all + positional via -- should be rejected");
        assert!(matches!(err, Error::InvalidArgument(_)));
    }

    #[test]
    fn all_accepts_pass_through_flags() {
        // `--all` + flag-only pass-through (`--silent`, `--loglevel=warn`) is
        // legitimate — they're PM-level flags, not positionals.
        let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.32.0");
        let extras = vec!["--silent".to_string(), "--loglevel=warn".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                all: true,
                pass_through_args: Some(&extras),
                ..Default::default()
            })
            .expect("flag-only extras allowed with --all")
            .expect("supported");
        assert_eq!(result.args, vec!["approve-builds", "--all", "--silent", "--loglevel=warn"]);
    }

    #[test]
    fn npm_warns_about_dropped_pass_through() {
        // No-op paths should not silently swallow user-supplied pass-through
        // args; they should at least surface a warning.
        let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
        let extras = vec!["--silent".to_string()];
        let result = pm
            .resolve_approve_builds_command(&ApproveBuildsCommandOptions {
                pass_through_args: Some(&extras),
                ..Default::default()
            })
            .expect("resolves");
        assert!(result.is_none()); // still a no-op
        // Visual inspection of the warn text is captured in snap-tests.
    }
}
