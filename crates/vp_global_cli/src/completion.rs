use std::ffi::OsString;

use usage_rs::complete::{Candidate, Completions, Files, Request, Shell};

use crate::{apply_chdir, cli::command_with_help, commands};

fn script_request(args: &[String]) -> Option<String> {
    if args.first().map(String::as_str) != Some("__complete_script__") {
        return None;
    }
    let shell = args
        .windows(2)
        .find_map(|pair| (pair[0] == "--shell").then(|| Shell::from_name(&pair[1])).flatten())
        .unwrap_or(Shell::Bash);
    let alias = args.windows(2).find_map(|pair| (pair[0] == "--alias").then_some(pair[1].as_str()));
    Some(alias.map_or_else(
        || usage_rs::script::script("vp", shell),
        |alias| usage_rs::script::script_for("vp", alias, shell),
    ))
}

fn local_request_args(args: &[String]) -> Vec<String> {
    let mut args = args.to_vec();
    if let Some(index) = args.iter().position(|arg| arg == "--shell") {
        if let Some(shell) = args.get_mut(index + 1) {
            *shell = "nu".to_owned();
        }
    } else {
        args.extend(["--shell".to_owned(), "nu".to_owned()]);
    }
    args
}

fn local_completions(output: &[u8]) -> Completions<'static> {
    let mut answer = Completions::default();
    for line in String::from_utf8_lossy(output).lines() {
        match line {
            usage_rs::complete::FILES_MARKER => answer.files = Some(Files::Any),
            usage_rs::complete::DIRS_MARKER => answer.files = Some(Files::Dirs),
            usage_rs::complete::EXECUTABLE_PATHS_MARKER => {
                answer.files = Some(Files::ExecutablePaths);
            }
            usage_rs::complete::COMMANDS_MARKER => answer.files = Some(Files::Commands),
            "" => {}
            value => {
                let (value, description) = value
                    .split_once('\t')
                    .map_or((value, None), |(value, description)| (value, Some(description)));
                answer.candidates.push(Candidate {
                    value: value.to_owned(),
                    description: description
                        .filter(|value| !value.is_empty())
                        .map(|value| value.to_owned().into()),
                });
            }
        }
    }
    answer
}

fn global_completions(request: &Request) -> Completions<'static> {
    let args = request.split.words.iter().map(OsString::from).collect::<Vec<_>>();
    let current_dir = std::env::current_dir().ok();
    let candidates = clap_complete::engine::complete(
        &mut command_with_help(),
        args,
        request.split.cword,
        current_dir.as_deref(),
    )
    .unwrap_or_default()
    .into_iter()
    .map(|candidate| Candidate {
        value: candidate.get_value().to_string_lossy().into_owned(),
        description: candidate.get_help().map(|help| help.to_string().into()),
    })
    .collect();
    Completions { candidates, files: None }
}

fn apply_request_chdir(request: &Request) {
    let Some(words) = request.split.words.get(1..) else {
        return;
    };
    let Some((dir, _)) = crate::parse_leading_chdir(words) else {
        return;
    };
    if let Ok(cwd) = vt_path::current_dir() {
        let _ = apply_chdir(&cwd, &dir);
    }
}

pub(crate) async fn request(args: &[String]) -> Option<String> {
    if let Some(script) = script_request(args) {
        return Some(script);
    }
    let argv = args.iter().map(OsString::from).collect::<Vec<_>>();
    let request = Request::parse(&argv)?;
    apply_request_chdir(&request);

    let mut answer = global_completions(&request);
    if let Ok(cwd) = vt_path::current_dir()
        && let Ok(output) = commands::delegate::execute_output(
            cwd,
            "__complete_word__",
            &local_request_args(&args[1..]),
        )
        .await
        && output.status.success()
    {
        let local = local_completions(&output.stdout);
        answer.candidates.extend(local.candidates);
        answer.files = answer.files.or(local.files);
    }
    answer.candidates.sort();
    answer.candidates.dedup_by(|left, right| left.value == right.value);
    Some(usage_rs::complete::render(&answer, request.shell))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_local_candidates_descriptions_and_markers() {
        let answer = local_completions(
            format!("run\tRun tasks\n--help\n{}\n", usage_rs::complete::DIRS_MARKER).as_bytes(),
        );
        assert_eq!(
            answer.candidates.iter().map(|candidate| candidate.value.as_str()).collect::<Vec<_>>(),
            ["run", "--help"]
        );
        assert_eq!(answer.candidates[0].description.as_deref(), Some("Run tasks"));
        assert_eq!(answer.candidates[1].description, None);
        assert_eq!(answer.files, Some(Files::Dirs));
    }

    #[test]
    fn generates_all_supported_scripts() {
        for shell in ["bash", "zsh", "fish", "nu", "powershell"] {
            let script = script_request(&[
                "__complete_script__".to_owned(),
                "--shell".to_owned(),
                shell.to_owned(),
            ])
            .expect("script request");
            assert!(script.contains("__complete_word__"), "{shell}");

            let alias = script_request(&[
                "__complete_script__".to_owned(),
                "--shell".to_owned(),
                shell.to_owned(),
                "--alias".to_owned(),
                "vpr".to_owned(),
            ])
            .expect("alias script request");
            assert!(alias.contains("vpr"), "{shell}");
        }
    }

    #[test]
    fn completes_global_commands_without_the_local_package() {
        let argv = [
            OsString::from("__complete_word__"),
            OsString::from("--shell"),
            OsString::from("bash"),
            OsString::from("--line"),
            OsString::from("vp en"),
        ];
        let request = Request::parse(&argv).expect("completion request");
        assert!(
            global_completions(&request)
                .candidates
                .iter()
                .any(|candidate| candidate.value == "env")
        );
    }
}
