use std::{collections::BTreeMap, ffi::OsStr, fmt::Display, path::Path, sync::Arc};

use crate::{config::TaskNode, fs::FileSystem, schedule::ExecutedTask, str::Str};
use bincode::{Decode, Encode};
use relative_path::RelativePath;
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum PathFingerprint {
    NotFound,
    FileContentHash(u64),
}

#[derive(Encode, Decode, Debug, Serialize, Deserialize)]
pub struct TaskFingerprint {
    pub command: Str,
    pub config_inputs: Arc<[Str]>,
    pub inputs: BTreeMap<Str, PathFingerprint>,
}

#[derive(Debug)]
pub enum FingerprintMismatch {
    ConfigInputsChanged { old_inputs: Arc<[Str]>, new_inputs: Arc<[Str]> },
    CommandChanged { old_command: Str, new_command: Str },
    InputContentChanged { path: Str },
}

impl Display for FingerprintMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FingerprintMismatch::ConfigInputsChanged { old_inputs, new_inputs } => {
                write!(f, "Config inputs changed: {:?} => {:?}", old_inputs, new_inputs)
            }
            FingerprintMismatch::CommandChanged { old_command, new_command } => {
                write!(f, "Command changed: {:?} => {:?}", old_command, new_command)
            }
            FingerprintMismatch::InputContentChanged { path } => {
                write!(f, "File content changed: {:?}", path)
            }
        }
    }
}

impl TaskFingerprint {
    pub fn create(
        task: &TaskNode,
        executed_task: &ExecutedTask,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> anyhow::Result<Self> {
        let command = task.command.clone();
        let mut inputs = BTreeMap::new();

        for input_full_path in &executed_task.input_paths {
            let Ok(relative_path) = Path::new(input_full_path).strip_prefix(base_dir) else {
                continue; // skip inputs outside the base_dir
            };
            let relative_path = RelativePath::from_path(relative_path)?.as_str();
            let path_fingerprint = fs.fingerprint_path(input_full_path)?;
            inputs.insert(relative_path.into(), path_fingerprint);
        }

        Ok(Self { command, inputs, config_inputs: task.inputs.clone() })
    }
    pub fn validate(
        &self,
        task: &TaskNode,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> anyhow::Result<Option<FingerprintMismatch>> {
        if self.command != task.command {
            return Ok(Some(FingerprintMismatch::CommandChanged {
                old_command: self.command.clone(),
                new_command: task.command.clone(),
            }));
        }
        if self.config_inputs != task.inputs {
            return Ok(Some(FingerprintMismatch::ConfigInputsChanged {
                old_inputs: self.config_inputs.clone(),
                new_inputs: task.inputs.clone(),
            }));
        }
        for (input_relative_path, path_fingerprint) in &self.inputs {
            let input_full_path =
                Arc::<OsStr>::from(base_dir.join(input_relative_path).into_os_string());
            let current_path_fingerprint = fs.fingerprint_path(&input_full_path)?;
            if path_fingerprint != &current_path_fingerprint {
                return Ok(Some(FingerprintMismatch::InputContentChanged {
                    path: input_relative_path.clone(),
                }));
            }
        }
        Ok(None)
    }
}
