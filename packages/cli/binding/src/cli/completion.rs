use std::{ffi::OsString, sync::Arc};

use rustc_hash::FxHashMap;
use usage_rs::complete::{Candidate, Completions, Request, Shell, Split};
use vt::{CommandHandler, CompletionData, HandledCommand, ScriptCommand, Session, SessionConfig};
use vt_path::{AbsolutePath, AbsolutePathBuf};
use vt_str::Str;

use super::{
    handler::VitePlusConfigLoader,
    types::{CliOptions, LocalCli},
};

#[derive(Debug)]
struct CompletionCommandHandler;

#[async_trait::async_trait(?Send)]
impl CommandHandler for CompletionCommandHandler {
    async fn handle_command(
        &mut self,
        _command: &mut ScriptCommand,
    ) -> anyhow::Result<HandledCommand> {
        Ok(HandledCommand::Verbatim)
    }
}

fn command_name<'a>(spec: &'a usage_rs::spec::Spec<'a>) -> &'a str {
    spec.bin.unwrap_or(spec.name).split_ascii_whitespace().next_back().unwrap_or(spec.name)
}

fn remove_leading_chdir(split: &mut Split) {
    if split.words.len() < 2 || split.cword <= 1 {
        return;
    }
    let remove = if split.words[1] == "-C" && split.words.len() > 2 && split.cword > 2 {
        2
    } else if split.words[1].starts_with("-C") && split.words[1].len() > 2 {
        1
    } else {
        0
    };
    if remove > 0 {
        split.words.drain(1..=remove);
        split.cword -= remove;
    }
}

fn merge_root_candidates(split: &Split) -> Completions<'static> {
    let mut answer = Completions::default();
    for spec in [vt::Cli::spec(), vp_pm_cli::PackageManagerCli::spec(), LocalCli::spec()] {
        answer.candidates.extend(usage_rs::complete::complete(spec, split).candidates);
    }
    for spec in crate::js_command_args::command_specs() {
        let name = command_name(spec);
        if name.starts_with(&split.prefix) {
            answer.candidates.push(Candidate { value: name.to_owned(), description: None });
        }
    }
    answer.candidates.sort();
    answer.candidates.dedup_by(|left, right| left.value == right.value);
    answer
}

fn is_vpr(split: &Split) -> bool {
    split.words.first().is_some_and(|word| {
        std::path::Path::new(word).file_stem().is_some_and(|name| name.eq_ignore_ascii_case("vpr"))
    })
}

fn complete_selected(split: &Split, data: &CompletionData) -> Completions<'static> {
    if is_vpr(split) {
        return vt::complete(split, data);
    }

    let Some(command) = split.words.get(1).map(String::as_str) else {
        return merge_root_candidates(split);
    };
    if super::command_matches(vt::Cli::spec(), command) {
        return vt::complete(split, data);
    }
    for spec in [vp_pm_cli::PackageManagerCli::spec(), LocalCli::spec()] {
        if super::command_matches(spec, command) {
            return usage_rs::complete::complete(spec, split);
        }
    }
    if let Some(spec) = crate::js_command_args::command_specs()
        .into_iter()
        .find(|spec| command_name(spec) == command)
    {
        let mut projected = split.clone();
        projected.words.remove(1);
        projected.cword = projected.cword.saturating_sub(1);
        return usage_rs::complete::complete(spec, &projected);
    }
    Completions::default()
}

async fn completion_data(cwd: &AbsolutePathBuf, options: Option<&CliOptions>) -> CompletionData {
    let Some(resolve) = options.map(|options| Arc::clone(&options.resolve_universal_vite_config))
    else {
        return CompletionData::default();
    };
    let envs = std::env::vars_os()
        .map(|(key, value)| (Arc::from(key.as_os_str()), Arc::from(value.as_os_str())))
        .collect::<FxHashMap<Arc<std::ffi::OsStr>, Arc<std::ffi::OsStr>>>();
    let mut command_handler = CompletionCommandHandler;
    let mut config_loader = VitePlusConfigLoader::new(resolve);
    let config = SessionConfig {
        command_handler: &mut command_handler,
        user_config_loader: &mut config_loader,
        program_name: Str::from("vp"),
    };
    let Ok(mut session) = Session::init_with(envs, Arc::<AbsolutePath>::from(cwd.clone()), config)
    else {
        return CompletionData::default();
    };
    session.completion_data().await.unwrap_or_default()
}

pub(super) async fn request(
    cwd: &AbsolutePathBuf,
    options: Option<&CliOptions>,
    args: &[String],
) -> Option<String> {
    let argv = args.iter().map(OsString::from).collect::<Vec<_>>();
    let mut request = Request::parse(&argv)?;
    remove_leading_chdir(&mut request.split);
    let data = if vt::completion_uses_workspace_data(&request.split) {
        completion_data(cwd, options).await
    } else {
        CompletionData::default()
    };
    let answer = if request.split.cword <= 1 && !is_vpr(&request.split) {
        merge_root_candidates(&request.split)
    } else {
        complete_selected(&request.split, &data)
    };
    // The global binary parses this stable tab-delimited form and renders the user's shell.
    Some(usage_rs::complete::render(&answer, Shell::Nu))
}

#[cfg(test)]
mod tests {
    use usage_rs::complete::{Request, Shell};

    use super::*;

    fn values(line: &str) -> Vec<String> {
        let argv = [
            OsString::from("__complete_word__"),
            OsString::from("--shell"),
            OsString::from("bash"),
            OsString::from("--line"),
            OsString::from(line),
        ];
        let mut request = Request::parse(&argv).expect("completion request");
        remove_leading_chdir(&mut request.split);
        let answer = if request.split.cword <= 1 && !is_vpr(&request.split) {
            merge_root_candidates(&request.split)
        } else {
            complete_selected(&request.split, &CompletionData::default())
        };
        usage_rs::complete::render(&answer, Shell::Bash).lines().map(ToOwned::to_owned).collect()
    }

    #[test]
    fn completes_all_local_command_sources() {
        assert!(values("vp cr").contains(&"create".to_owned()));
        assert!(values("vp ru").contains(&"run".to_owned()));
        assert!(values("vp in").contains(&"install".to_owned()));
        assert!(values("vp li").contains(&"lint".to_owned()));
    }

    #[test]
    fn completes_javascript_command_options() {
        assert!(values("vp staged --di").contains(&"--diff".to_owned()));
        assert!(values("vp create --package-manager p").contains(&"pnpm".to_owned()));
    }

    #[test]
    fn completes_vite_task_options_and_vpr_view() {
        assert!(values("vp run --lo").contains(&"--log".to_owned()));
        assert!(values("vpr --lo").contains(&"--log".to_owned()));
        assert!(!values("vp run build -- --lo").contains(&"--log".to_owned()));
    }

    #[test]
    fn removes_the_node_owned_chdir_option() {
        assert!(values("vp -C workspace staged --di").contains(&"--diff".to_owned()));
        assert!(values("vp -Cworkspace staged --di").contains(&"--diff".to_owned()));
    }
}
