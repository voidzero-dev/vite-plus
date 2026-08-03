use std::{collections::HashMap, process::ExitStatus};

use vite_path::AbsolutePath;

use crate::{
    Error,
    resolution::{
        CommandResolution, Resolution,
        command::{PreRunAction, ResolvedCommand},
    },
};

pub(crate) async fn run_resolution(
    cwd: &AbsolutePath,
    resolution: Resolution,
    render_diagnostics: bool,
) -> Result<ExitStatus, Error> {
    if render_diagnostics && !resolution.diagnostics.is_empty() {
        resolution.diagnostics.render();
    }

    match resolution.outcome {
        CommandResolution::Run(command) => run_command(cwd, command).await,
        CommandResolution::Noop => Ok(ExitStatus::default()),
        CommandResolution::InvalidArgument(message) => Err(Error::UserMessage(message.into())),
    }
}

async fn run_command(cwd: &AbsolutePath, command: ResolvedCommand) -> Result<ExitStatus, Error> {
    let ResolvedCommand { program, args, env, pre_run } = command;
    run_pre_run_actions(cwd, pre_run).await?;

    let env = env.into_iter().collect::<HashMap<_, _>>();
    Ok(vite_command::run_command(&program, args, &env, cwd).await?)
}

async fn run_pre_run_actions(cwd: &AbsolutePath, actions: Vec<PreRunAction>) -> Result<(), Error> {
    for action in actions {
        match action {
            PreRunAction::CreateDir { path } => {
                let path = cwd.join(path);
                tokio::fs::create_dir_all(&path).await.map_err(|err| {
                    Error::Install(vite_error::Error::IoWithPath { path: path.into(), err })
                })?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use vite_path::AbsolutePathBuf;

    use super::*;
    use crate::resolution::{Diagnostics, command::ResolvedCommand};

    fn resolution(outcome: CommandResolution) -> Resolution {
        Resolution { outcome, diagnostics: Diagnostics::default() }
    }

    #[tokio::test]
    async fn noop_returns_success() {
        let cwd = vite_path::current_dir().unwrap();

        let status = run_resolution(&cwd, resolution(CommandResolution::Noop), true).await.unwrap();

        assert!(status.success());
    }

    #[tokio::test]
    async fn invalid_argument_becomes_user_message() {
        let cwd = vite_path::current_dir().unwrap();

        let error = run_resolution(
            &cwd,
            resolution(CommandResolution::InvalidArgument("invalid option".to_string())),
            true,
        )
        .await
        .unwrap_err();

        assert!(matches!(error, Error::UserMessage(message) if message == "invalid option"));
    }

    #[tokio::test]
    async fn create_dir_is_relative_to_caller_cwd() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let action = PreRunAction::CreateDir { path: "output/nested".to_string() };

        run_pre_run_actions(&cwd, vec![action]).await.unwrap();

        assert!(cwd.join("output/nested").as_path().is_dir());
    }

    #[tokio::test]
    async fn pre_run_actions_execute_before_spawn() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cwd = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();
        let mut command = ResolvedCommand::new("vite-plus-command-that-does-not-exist");
        command.pre_run.push(PreRunAction::CreateDir { path: "created-before-spawn".to_string() });

        let error = run_resolution(&cwd, resolution(CommandResolution::Run(command)), true)
            .await
            .unwrap_err();

        assert!(matches!(error, Error::Install(_)));
        assert!(cwd.join("created-before-spawn").as_path().is_dir());
    }
}
