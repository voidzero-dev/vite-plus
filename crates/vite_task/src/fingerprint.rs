use std::{ffi::OsStr, fmt::Display, path::Path, sync::Arc};

use crate::{
    collections::HashMap,
    config::{
        CommandFingerprint, CommandFingerprintDiff, ResolvedTask, ResolvedTaskConfig,
        ResolvedTaskConfigDiff,
    },
    execute::ExecutedTask,
    fs::FileSystem,
    str::Str,
};

use bincode::{Decode, Encode};
use diff::Diff as _;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use relative_path::RelativePath;
use serde::{Deserialize, Serialize};

/// The fingerprint of a task. Determines if the task needs to be re-executed
#[derive(Encode, Decode, Debug, Serialize)]
pub struct TaskFingerprint {
    pub resolved_config: ResolvedTaskConfig,
    pub command_fingerprint: CommandFingerprint,
    pub inputs: HashMap<Str, PathFingerprint>,
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum PathFingerprint {
    NotFound,
    FileContentHash(u64),
}

#[derive(Debug)]
pub enum FingerprintMismatch {
    ConfigChanged(ResolvedTaskConfigDiff),
    InputContentChanged { path: Str },
    ResolvedCommandChanged(CommandFingerprintDiff),
}

impl Display for FingerprintMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigChanged(config_diff) => {
                write!(f, "Config inputs changed: {config_diff:?}")
            }
            Self::InputContentChanged { path } => {
                write!(f, "File content changed: {path:?}")
            }
            Self::ResolvedCommandChanged(env_diff) => {
                write!(f, "Resolved command changed: {env_diff:?}")
            }
        }
    }
}

impl TaskFingerprint {
    /// Checks if the cached fingerprint is still valid. Returns why if not.
    pub fn validate(
        &self,
        resolved_task: &ResolvedTask,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> anyhow::Result<Option<FingerprintMismatch>> {
        // TODO: use diff result instead of eq
        Ok(if self.resolved_config != resolved_task.resolved_config {
            Some(FingerprintMismatch::ConfigChanged(
                self.resolved_config.diff(&resolved_task.resolved_config),
            ))
        } else if self.command_fingerprint != resolved_task.resolved_command.fingerprint {
            Some(FingerprintMismatch::ResolvedCommandChanged(
                self.command_fingerprint.diff(&resolved_task.resolved_command.fingerprint),
            ))
        } else {
            let input_mismatch =
                self.inputs.par_iter().find_map_any(|(input_relative_path, path_fingerprint)| {
                    let input_full_path =
                        Arc::<OsStr>::from(base_dir.join(input_relative_path).into_os_string());
                    let current_path_fingerprint = match fs.fingerprint_path(&input_full_path) {
                        Ok(ok) => ok,
                        Err(err) => return Some(Err(err.into())),
                    };
                    if path_fingerprint == &current_path_fingerprint {
                        None
                    } else {
                        Some(anyhow::Ok(FingerprintMismatch::InputContentChanged {
                            path: input_relative_path.clone(),
                        }))
                    }
                });
            input_mismatch.transpose()?
        })
    }

    /// Creates a new fingerprint after the task has been executed
    pub fn create(
        task: ResolvedTask,
        executed_task: &ExecutedTask,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> anyhow::Result<Self> {
        let inputs = executed_task
            .input_paths
            .par_iter()
            .flat_map(|input_full_path| {
                let Ok(relative_path) = Path::new(input_full_path).strip_prefix(base_dir) else {
                    return None; // skip inputs outside the base_dir
                };
                Some((|| {
                    let relative_path = RelativePath::from_path(relative_path)?.as_str();
                    let path_fingerprint = fs.fingerprint_path(input_full_path)?;
                    anyhow::Ok((relative_path.into(), path_fingerprint))
                })())
            })
            .collect::<anyhow::Result<HashMap<Str, PathFingerprint>>>()?;
        Ok(Self {
            resolved_config: task.resolved_config,
            command_fingerprint: task.resolved_command.fingerprint,
            inputs,
        })
    }
}
