use std::{
    collections::HashMap,
    fmt,
    process::ExitStatus,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};

use dialoguer::{
    MultiSelect,
    theme::{ColorfulTheme, Theme},
};
use serde::Deserialize;
use vt_path::AbsolutePath;

use crate::{
    Error,
    resolution::command::{PnpmInteractiveUpdate, ResolvedCommand},
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct OutdatedPackage {
    current: Option<String>,
    wanted: Option<String>,
    latest: Option<String>,
    dependency_type: Option<String>,
}

struct UpdateChoice {
    package: String,
    label: vt_str::Str,
    github_action: bool,
}

pub(super) async fn run_pnpm_interactive_update(
    cwd: &AbsolutePath,
    plan: PnpmInteractiveUpdate,
) -> Result<ExitStatus, Error> {
    let output = capture(cwd, &plan.outdated).await?;
    if !output.status.success() && vp_shared::exit_code_from_status(output.status) != 1 {
        if !output.stdout.is_empty() {
            vp_shared::output::raw(&String::from_utf8_lossy(&output.stdout));
        }
        return Ok(output.status);
    }

    let outdated: HashMap<String, OutdatedPackage> = serde_json::from_slice(&output.stdout)?;
    let choices = update_choices(outdated, plan.latest, plan.include_github_actions);
    if choices.is_empty() {
        if plan.latest {
            vp_shared::output::raw("All of your dependencies are already up to date");
        } else {
            vp_shared::output::raw(
                "All of your dependencies are already up to date inside the specified ranges. \
                 Use the --latest option to update the ranges in package.json",
            );
        }
        return Ok(ExitStatus::default());
    }

    let theme = InteractiveUpdateTheme::new(choices.len());
    let selected = MultiSelect::with_theme(&theme)
        .with_prompt("Choose which dependencies to update")
        .items(choices.iter().map(|choice| choice.label.as_str()).collect::<Vec<_>>())
        .interact()
        .map_err(|error| {
            Error::Other(vt_str::format!("interactive update selection failed: {error}"))
        })?;

    if selected.is_empty() {
        return Ok(ExitStatus::default());
    }

    let mut update = plan.update;
    if selected.iter().any(|&index| choices[index].github_action)
        && !update.args.iter().any(|arg| arg == "--include-github-actions")
    {
        update.args.push("--include-github-actions".to_string());
    }
    update.args.extend(selected.into_iter().map(|index| choices[index].package.clone()));
    run(cwd, update).await
}

async fn capture(
    cwd: &AbsolutePath,
    command: &ResolvedCommand,
) -> Result<std::process::Output, Error> {
    let env = command.env.clone().into_iter().collect::<HashMap<_, _>>();
    vp_command::capture_stdout(&command.program, &command.args, &env, cwd).await.map_err(Into::into)
}

async fn run(cwd: &AbsolutePath, command: ResolvedCommand) -> Result<ExitStatus, Error> {
    let env = command.env.into_iter().collect::<HashMap<_, _>>();
    Ok(vp_command::run_command(&command.program, command.args, &env, cwd).await?)
}

fn update_choices(
    outdated: HashMap<String, OutdatedPackage>,
    latest: bool,
    include_github_actions: bool,
) -> Vec<UpdateChoice> {
    let mut outdated = outdated.into_iter().collect::<Vec<_>>();
    outdated.sort_by(|(left, _), (right, _)| left.cmp(right));

    let rows = outdated
        .into_iter()
        .filter_map(|(package, details)| {
            let github_action = details.dependency_type.as_deref() == Some("githubAction");
            if github_action && !include_github_actions {
                return None;
            }
            let target = if latest { details.latest } else { details.wanted }?;
            if details.current.as_deref() == Some(target.as_str()) {
                return None;
            }
            Some((
                package,
                details.current.unwrap_or_default(),
                target,
                details.dependency_type,
                github_action,
            ))
        })
        .collect::<Vec<_>>();
    let package_width = rows.iter().map(|(package, ..)| package.len()).max().unwrap_or_default();
    let current_width = rows.iter().map(|(_, current, ..)| current.len()).max().unwrap_or_default();
    let target_width = rows.iter().map(|(_, _, target, ..)| target.len()).max().unwrap_or_default();

    rows.into_iter()
        .map(|(package, current, target, dependency_type, github_action)| {
            let group =
                dependency_type.map(|group| vt_str::format!("[{group}] ")).unwrap_or_default();
            let url = if github_action {
                vt_str::format!("https://github.com/{package}")
            } else {
                npmx_changelog_url(&package, &target)
            };
            let label = vt_str::format!(
                "{group}{package:package_width$} {current:>current_width$} ❯ {target:target_width$} {url}"
            );
            UpdateChoice { package, label, github_action }
        })
        .collect()
}

fn npmx_changelog_url(package: &str, version: &str) -> vt_str::Str {
    let package = form_urlencoded::byte_serialize(package.as_bytes()).collect::<String>();
    let version = form_urlencoded::byte_serialize(version.as_bytes()).collect::<String>();
    vt_str::format!("https://npmx.dev/package-changelog/{package}/v/{version}")
}

/// A `dialoguer` theme that lets the PTY runner synchronize only after the
/// first complete multi-select frame has been written.
struct InteractiveUpdateTheme {
    inner: ColorfulTheme,
    items_before_milestone: AtomicUsize,
}

impl InteractiveUpdateTheme {
    fn new(item_count: usize) -> Self {
        Self {
            inner: ColorfulTheme::default(),
            items_before_milestone: AtomicUsize::new(item_count),
        }
    }

    fn rendered_last_initial_item(&self) -> bool {
        let mut remaining = self.items_before_milestone.load(Ordering::Relaxed);
        loop {
            if remaining == 0 {
                return false;
            }
            match self.items_before_milestone.compare_exchange_weak(
                remaining,
                remaining - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return remaining == 1,
                Err(actual) => remaining = actual,
            }
        }
    }
}

impl Theme for InteractiveUpdateTheme {
    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.inner.format_multi_select_prompt(f, prompt)
    }

    fn format_multi_select_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selections: &[&str],
    ) -> fmt::Result {
        self.inner.format_multi_select_prompt_selection(f, prompt, selections)
    }

    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        self.inner.format_multi_select_prompt_item(f, text, checked, active)?;

        if self.rendered_last_initial_item() {
            write_prompt_milestone(f)?;
        }

        Ok(())
    }
}

/// Write an invisible window-title marker into the rendered frame. Dialoguer
/// flushes the frame before reading a key, so the runner cannot race the prompt.
fn write_prompt_milestone(f: &mut dyn fmt::Write) -> fmt::Result {
    if std::env::var_os(vp_shared::env_vars::VP_EMIT_MILESTONES).is_none_or(|value| value != "1") {
        return Ok(());
    }

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = (u128::from(std::process::id()) << 64)
        | u128::from(COUNTER.fetch_add(1, Ordering::Relaxed));
    let encoded = base64_simd::URL_SAFE_NO_PAD.encode_to_string(b"multi-select:update:ready");
    write!(f, "\x1b]2;pty-terminal-test:{id:032x}:{encoded}\x1b\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_version_specific_npmx_changelog_links() {
        let outdated = HashMap::from([
            (
                "vite".to_string(),
                OutdatedPackage {
                    current: Some("7.0.0".to_string()),
                    wanted: Some("7.2.0".to_string()),
                    latest: Some("8.2.1".to_string()),
                    dependency_type: Some("devDependencies".to_string()),
                },
            ),
            (
                "@vitejs/plugin-react".to_string(),
                OutdatedPackage {
                    current: Some("5.0.0".to_string()),
                    wanted: Some("5.1.4".to_string()),
                    latest: Some("6.0.1".to_string()),
                    dependency_type: Some("devDependencies".to_string()),
                },
            ),
        ]);

        let choices = update_choices(outdated, false, true);

        assert_eq!(choices.len(), 2);
        assert_eq!(choices[0].package, "@vitejs/plugin-react");
        assert!(choices[0].label.contains("@vitejs/plugin-react"));
        assert!(
            choices[0]
                .label
                .contains("https://npmx.dev/package-changelog/%40vitejs%2Fplugin-react/v/5.1.4")
        );
        assert!(choices[1].label.contains("https://npmx.dev/package-changelog/vite/v/7.2.0"));
    }

    #[test]
    fn latest_uses_latest_target_and_omits_current_versions() {
        let outdated = HashMap::from([
            (
                "already-current".to_string(),
                OutdatedPackage {
                    current: Some("2.0.0".to_string()),
                    wanted: Some("2.0.0".to_string()),
                    latest: Some("2.0.0".to_string()),
                    dependency_type: None,
                },
            ),
            (
                "next".to_string(),
                OutdatedPackage {
                    current: Some("1.0.0".to_string()),
                    wanted: Some("1.1.0".to_string()),
                    latest: Some("3.0.0".to_string()),
                    dependency_type: None,
                },
            ),
        ]);

        let choices = update_choices(outdated, true, true);

        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].package, "next");
        assert!(choices[0].label.contains("/next/v/3.0.0"));
    }

    #[test]
    fn github_actions_keep_their_repository_link_and_can_be_filtered() {
        let outdated = HashMap::from([(
            "actions/checkout".to_string(),
            OutdatedPackage {
                current: Some("4".to_string()),
                wanted: Some("4".to_string()),
                latest: Some("5".to_string()),
                dependency_type: Some("githubAction".to_string()),
            },
        )]);

        assert!(update_choices(outdated.clone(), true, false).is_empty());
        let choices = update_choices(outdated, true, true);
        assert_eq!(choices.len(), 1);
        assert!(choices[0].github_action);
        assert!(choices[0].label.contains("https://github.com/actions/checkout"));
        assert!(!choices[0].label.contains("npmx.dev"));
    }
}
