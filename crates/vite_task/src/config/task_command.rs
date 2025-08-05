use std::{fmt::Display, path::Path};

use crate::{Error, cmd::TaskParsedCommand, execute::TaskEnvs, str::Str};

use bincode::{Decode, Encode};
use diff::Diff;
use relative_path::RelativePath;
use serde::{Deserialize, Serialize};

use super::{CommandFingerprint, ResolvedTaskCommand, TaskConfig};

#[derive(Encode, Decode, Serialize, Deserialize, Debug, PartialEq, Eq, Diff, Clone)]
#[diff(attr(#[derive(Debug)]))]
#[serde(untagged)]
pub enum TaskCommand {
    ShellScript(Str),
    #[serde(skip_deserializing)]
    Parsed(TaskParsedCommand),
}

impl Display for TaskCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ShellScript(command) => Display::fmt(&command, f),
            Self::Parsed(parsed_command) => Display::fmt(&parsed_command, f),
        }
    }
}

impl From<TaskCommand> for TaskConfig {
    fn from(command: TaskCommand) -> Self {
        Self {
            command,
            cwd: "".into(),
            cacheable: true,
            inputs: Default::default(),
            envs: Default::default(),
            pass_through_envs: Default::default(),
        }
    }
}

#[derive(Encode, Decode, Debug, Serialize, PartialEq, Eq, Diff, Clone)]
#[diff(attr(#[derive(Debug)]))]
pub struct ResolvedTaskConfig {
    pub config_dir: Str,
    pub config: TaskConfig,
}

impl ResolvedTaskConfig {
    pub(crate) fn resolve_command(
        &self,
        base_dir: &Path,
        task_args: &[Str],
    ) -> Result<ResolvedTaskCommand, Error> {
        let cwd = RelativePath::new(&self.config_dir).join(self.config.cwd.as_str());
        let command = if task_args.is_empty() {
            self.config.command.clone()
        } else {
            match &self.config.command {
                TaskCommand::ShellScript(command_script) => {
                    let command_script =
                        std::iter::once(command_script.clone())
                            .chain(task_args.iter().map(|arg| {
                                shell_escape::escape(arg.as_str().into()).as_ref().into()
                            }))
                            .collect::<Vec<_>>()
                            .join(" ")
                            .into();
                    TaskCommand::ShellScript(command_script)
                }
                TaskCommand::Parsed(parsed_command) => {
                    let mut parsed_command = parsed_command.clone();
                    parsed_command.args.extend_from_slice(task_args);
                    TaskCommand::Parsed(parsed_command)
                }
            }
        };
        let task_envs = TaskEnvs::resolve(base_dir, self)?;
        Ok(ResolvedTaskCommand {
            fingerprint: CommandFingerprint {
                cwd: cwd.as_str().into(),
                command,
                envs_without_pass_through: task_envs.envs_without_pass_through,
            },
            all_envs: task_envs.all_envs,
        })
    }
}
