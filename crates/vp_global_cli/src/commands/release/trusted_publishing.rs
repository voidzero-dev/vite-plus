//! Registry-side trusted-publisher setup for workspace packages.
//!
//! npm is currently the only npm-registry client that exposes the configuration API (`npm
//! trust`). pnpm and modern Yarn can consume the resulting OIDC relationship during native
//! publish, while Bun packages are packed by Bun and handed to npm for the authenticated publish.

use std::{io::IsTerminal, process::ExitStatus, time::Duration};

use super::{
    AbsolutePath, Error, FirstPublishGuidance, OidcPublishTransport, PackageManagerType,
    ReleaseOptions, TrustedPublishProvider, TrustedPublishingSetupOptions, WorkspacePackage,
    detect_github_repo, ensure_first_publish_workflow_template,
    find_existing_release_workflow_path, find_release_workflow_path,
    get_package_manager_type_and_version, load_workspace_packages, npm_for_trusted_publishing,
    oidc_publish_transport_for, output, parse_github_repo_slug, prepend_js_runtime_to_path_env,
    prompt_for_confirmation, push_joined, run_managed_command, select_workspace_packages,
    workflow_filename,
};

const BULK_TRUST_REQUEST_DELAY: Duration = Duration::from_secs(2);

/// Configures a GitHub Actions trusted publisher for every selected public workspace package.
pub(super) async fn execute_trusted_publishing_setup(
    cwd: &AbsolutePath,
    release_options: &ReleaseOptions,
    setup: &TrustedPublishingSetupOptions,
) -> Result<ExitStatus, Error> {
    validate_setup_release_options(release_options)?;

    let (workspace_root, _) = vt_workspace::find_workspace_root(cwd)?;
    let workspace_root_path = workspace_root.path.to_absolute_path_buf();
    let (project_package_manager, project_package_manager_version, _, _) =
        get_package_manager_type_and_version(&workspace_root, Some(PackageManagerType::Npm))?;
    let package_graph = vt_workspace::load_package_graph(&workspace_root)?;
    let workspace_packages = load_workspace_packages(&package_graph)?;
    let selected =
        select_workspace_packages(&workspace_packages, release_options.projects.as_deref())?;
    if selected.is_empty() {
        return Err(Error::UserMessage(
            "No publishable packages matched the trusted-publishing selection.".into(),
        ));
    }

    let repository = resolve_repository(&workspace_root_path, setup.repository.as_deref())?;
    let existing_workflow = find_existing_release_workflow_path(&workspace_root_path);
    let (workflow_path, should_scaffold_workflow) = match setup.workflow.as_deref() {
        Some(workflow) => {
            let workflow_path = normalize_workflow_path(workflow)?;
            if !workspace_root_path.join(&workflow_path).as_path().is_file() {
                let mut message = String::from("Trusted-publisher workflow does not exist: ");
                message.push_str(&workflow_path);
                message.push_str(". Create it first or omit --trusted-publisher-workflow to let vite-plus scaffold a default workflow.");
                return Err(Error::UserMessage(message.into()));
            }
            (workflow_path, false)
        }
        None => match existing_workflow {
            Some(workflow_path) => (workflow_path, false),
            None => (find_release_workflow_path(&workspace_root_path), true),
        },
    };
    let workflow_file = workflow_filename(&workflow_path).to_owned();
    validate_optional_value("--trusted-publisher-environment", setup.environment.as_deref())?;
    validate_optional_value("--trusted-publisher-registry", setup.registry.as_deref())?;

    print_setup_plan(
        &repository,
        &workflow_path,
        should_scaffold_workflow,
        &selected,
        project_package_manager,
        &project_package_manager_version,
        release_options,
        setup,
    );

    if !release_options.dry_run && !release_options.yes && !confirm_setup_interactively()? {
        output::note("Trusted-publishing setup cancelled.");
        return Ok(ExitStatus::default());
    }

    if should_scaffold_workflow && !release_options.dry_run {
        let mut guidance =
            FirstPublishGuidance { workflow_path: workflow_path.clone(), ..Default::default() };
        ensure_first_publish_workflow_template(
            &workspace_root_path,
            project_package_manager,
            setup.environment.as_deref(),
            &mut guidance,
        )?;
    }

    prepend_js_runtime_to_path_env(&workspace_root_path).await?;
    let npm = npm_for_trusted_publishing().await?;

    output::raw("");
    output::info(if release_options.dry_run {
        "Validating trusted-publisher configuration:"
    } else {
        "Configuring trusted publishers:"
    });

    for (index, package) in selected.iter().enumerate() {
        let mut line =
            String::from(if release_options.dry_run { "checking " } else { "configuring " });
        line.push_str(&package.name);
        output::note(&line);

        let args = trusted_publisher_args(
            &package.name,
            &repository,
            &workflow_file,
            release_options.dry_run,
            setup,
        );
        let status = run_managed_command(&workspace_root_path, &npm, &args).await?;
        if !status.success() {
            let mut message = String::from("Could not configure trusted publishing for ");
            message.push_str(&package.name);
            message.push_str(". The package must already exist on the registry, and the current npm account needs package write access with 2FA enabled. npm also rejects a second relationship; inspect it with `npm trust list ");
            message.push_str(&package.name);
            message.push_str("` and explicitly revoke the old relationship before replacing it. Packages after this one were not changed.");
            output::warn(&message);
            return Ok(status);
        }

        if index + 1 < selected.len() {
            // npm recommends spacing bulk trust changes to avoid registry rate limits.
            tokio::time::sleep(BULK_TRUST_REQUEST_DELAY).await;
        }
    }

    output::success(if release_options.dry_run {
        "Trusted-publisher configuration is valid; no registry changes were made."
    } else {
        "Trusted publishing is configured for every selected package."
    });
    if should_scaffold_workflow && !release_options.dry_run {
        output::note(
            "Commit and push the generated workflow before running the first OIDC release.",
        );
    }

    Ok(ExitStatus::default())
}

fn validate_setup_release_options(options: &ReleaseOptions) -> Result<(), Error> {
    let incompatible = options.skip_publish
        || options.first_release
        || options.changelog
        || options.version.is_some()
        || options.preid.is_some()
        || options.otp.is_some();
    if incompatible {
        return Err(Error::UserMessage(
            "`--setup-trusted-publishing` is a configuration-only mode. It can be combined with `--projects`, `--dry-run`, `--yes`, and trusted-publisher options, but not release-planning or publish options."
                .into(),
        ));
    }
    Ok(())
}

fn resolve_repository(cwd: &AbsolutePath, explicit: Option<&str>) -> Result<String, Error> {
    if let Some(repository) = explicit {
        return normalize_repository(repository);
    }
    detect_github_repo(cwd).ok_or_else(|| {
        Error::UserMessage(
            "Could not infer a GitHub repository from remote.origin.url. Pass --trusted-publisher-repository owner/repository."
                .into(),
        )
    })
}

fn normalize_repository(input: &str) -> Result<String, Error> {
    let input = input.trim().trim_end_matches('/');
    if let Some(repository) = parse_github_repo_slug(input) {
        return Ok(repository);
    }

    let input = input.strip_suffix(".git").unwrap_or(input);
    let mut parts = input.split('/');
    let owner = parts.next().unwrap_or_default();
    let repository = parts.next().unwrap_or_default();
    let valid_part = |part: &str| {
        !part.is_empty()
            && part != "."
            && part != ".."
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    };
    if !valid_part(owner) || !valid_part(repository) || parts.next().is_some() {
        return Err(Error::UserMessage(
            "--trusted-publisher-repository must be a GitHub repository in owner/repository form."
                .into(),
        ));
    }

    let mut normalized = String::with_capacity(owner.len() + repository.len() + 1);
    normalized.push_str(owner);
    normalized.push('/');
    normalized.push_str(repository);
    Ok(normalized)
}

fn normalize_workflow_path(input: &str) -> Result<String, Error> {
    let normalized = input.trim().replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    let file = if let Some(file) = normalized.strip_prefix(".github/workflows/") {
        file
    } else if !normalized.contains('/') {
        normalized
    } else {
        return Err(Error::UserMessage(
            "--trusted-publisher-workflow must be a filename or a path under .github/workflows."
                .into(),
        ));
    };
    let lowercase = file.to_ascii_lowercase();
    if file.is_empty()
        || file.contains('/')
        || file == "."
        || file == ".."
        || !(lowercase.ends_with(".yml") || lowercase.ends_with(".yaml"))
    {
        return Err(Error::UserMessage(
            "--trusted-publisher-workflow must name one .yml or .yaml file in .github/workflows."
                .into(),
        ));
    }

    let mut path = String::with_capacity(file.len() + 18);
    path.push_str(".github/workflows/");
    path.push_str(file);
    Ok(path)
}

fn validate_optional_value(flag: &str, value: Option<&str>) -> Result<(), Error> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        let mut message = String::from(flag);
        message.push_str(" cannot be empty.");
        return Err(Error::UserMessage(message.into()));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn print_setup_plan(
    repository: &str,
    workflow_path: &str,
    should_scaffold_workflow: bool,
    packages: &[WorkspacePackage],
    project_package_manager: PackageManagerType,
    project_package_manager_version: &str,
    release_options: &ReleaseOptions,
    setup: &TrustedPublishingSetupOptions,
) {
    output::raw("");
    output::info("Trusted-publishing setup:");
    output::note(&format!("repository: {repository}"));
    output::note(&format!("workflow: {workflow_path}"));
    if should_scaffold_workflow {
        output::note(if release_options.dry_run {
            "workflow action: would scaffold the built-in GitHub Actions publish workflow"
        } else {
            "workflow action: scaffold the built-in GitHub Actions publish workflow"
        });
    }
    if let Some(environment) = setup.environment.as_deref() {
        output::note(&format!("environment: {environment}"));
        if !should_scaffold_workflow {
            output::warn(
                "Ensure the publish job in the existing workflow uses this exact GitHub Actions environment.",
            );
        }
    }
    output::note(if setup.allow_stage_publish {
        "registry permissions: publish and stage publish"
    } else {
        "registry permissions: publish"
    });
    let mut package_line = String::from("packages: ");
    push_joined(&mut package_line, packages.iter().map(|package| package.name.as_str()), ", ");
    output::note(&package_line);

    let strategy = match oidc_publish_transport_for(
        project_package_manager,
        project_package_manager_version,
        Some(TrustedPublishProvider::GitHubActions),
    ) {
        OidcPublishTransport::Native => match project_package_manager {
            PackageManagerType::Npm => "npm publishes with native OIDC",
            PackageManagerType::Pnpm => "pnpm publishes with native OIDC",
            PackageManagerType::Yarn => "modern Yarn publishes with native OIDC",
            PackageManagerType::Bun => unreachable!("Bun does not provide native npm OIDC"),
        },
        OidcPublishTransport::ManagedNpm => "managed npm performs the OIDC publish directly",
        OidcPublishTransport::PackThenManagedNpm => {
            "the project package manager packs; managed npm publishes the immutable tarball via OIDC"
        }
    };
    output::note(&format!("release strategy: {strategy}"));
    output::note(
        "Packages must already exist on the registry before npm can attach a trusted publisher.",
    );
}

fn confirm_setup_interactively() -> Result<bool, Error> {
    if !std::io::stdin().is_terminal() {
        return Err(Error::UserMessage(
            "Cannot prompt for trusted-publishing confirmation: stdin is not a TTY. Use --yes to continue non-interactively."
                .into(),
        ));
    }
    prompt_for_confirmation("Configure these registry relationships? [y/N] ", false)
}

fn trusted_publisher_args(
    package: &str,
    repository: &str,
    workflow_file: &str,
    dry_run: bool,
    setup: &TrustedPublishingSetupOptions,
) -> Vec<String> {
    let mut args = Vec::with_capacity(16);
    args.extend(["trust", "github"].map(str::to_owned));
    args.push(package.to_owned());
    args.extend(["--file".to_owned(), workflow_file.to_owned()]);
    args.extend(["--repository".to_owned(), repository.to_owned()]);
    if let Some(environment) = setup.environment.as_deref() {
        args.extend(["--environment".to_owned(), environment.to_owned()]);
    }
    args.push("--allow-publish".to_owned());
    if setup.allow_stage_publish {
        args.push("--allow-stage-publish".to_owned());
    }
    if let Some(registry) = setup.registry.as_deref() {
        args.extend(["--registry".to_owned(), registry.to_owned()]);
    }
    if dry_run {
        args.push("--dry-run".to_owned());
    }
    args.push("--yes".to_owned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_accepts_slug_and_github_url() {
        assert_eq!(
            normalize_repository("voidzero-dev/vite-plus").unwrap(),
            "voidzero-dev/vite-plus"
        );
        assert_eq!(
            normalize_repository("https://github.com/voidzero-dev/vite-plus.git").unwrap(),
            "voidzero-dev/vite-plus"
        );
    }

    #[test]
    fn workflow_is_restricted_to_github_workflows() {
        assert_eq!(
            normalize_workflow_path("publish.yml").unwrap(),
            ".github/workflows/publish.yml"
        );
        assert_eq!(
            normalize_workflow_path(".github\\workflows\\release.yaml").unwrap(),
            ".github/workflows/release.yaml"
        );
        assert!(normalize_workflow_path("scripts/publish.yml").is_err());
        assert!(normalize_workflow_path("../publish.yml").is_err());
    }

    #[test]
    fn npm_trust_args_are_explicit_and_non_interactive() {
        let args = trusted_publisher_args(
            "@scope/package",
            "owner/repository",
            "publish.yml",
            true,
            &TrustedPublishingSetupOptions {
                environment: Some("production".into()),
                registry: Some("https://registry.npmjs.org".into()),
                allow_stage_publish: true,
                ..Default::default()
            },
        );

        assert_eq!(
            args,
            [
                "trust",
                "github",
                "@scope/package",
                "--file",
                "publish.yml",
                "--repository",
                "owner/repository",
                "--environment",
                "production",
                "--allow-publish",
                "--allow-stage-publish",
                "--registry",
                "https://registry.npmjs.org",
                "--dry-run",
                "--yes",
            ]
        );
    }
}
