use std::{collections::HashMap, ffi::OsStr, fmt::Display, path::{Path, PathBuf}, sync::Arc};

use crate::{
    config::{ResolvedTask, TaskConfig, TaskConfigDiff},
    execute::{ExecutedTask, TaskEnvs},
    fs::FileSystem,
    str::Str,
};

use bincode::{Decode, Encode};
use diff::{Diff as _, HashMapDiff};
use git2::Oid;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use relative_path::RelativePath;
use serde::{Deserialize, Serialize};
// use rayon::prelude::*;

#[derive(Encode, Decode, PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum PathFingerprint {
    NotFound,
    FileContentHash(u64),
}

#[derive(Encode, Decode, Debug, Serialize, Deserialize)]
pub struct TaskFingerprint {
    pub config: TaskConfig,
    pub inputs: HashMap<Str, PathFingerprint>,
    pub envs: HashMap<Str, Str>,
}

#[derive(Debug)]
pub enum FingerprintMismatch {
    ConfigChanged(TaskConfigDiff),
    InputContentChanged { path: Str },
    EnvChanged(HashMapDiff<Str, Str>),
}


impl Display for FingerprintMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FingerprintMismatch::ConfigChanged(config_diff) => {
                write!(f, "Config inputs changed: {:?}", config_diff)
            }
            FingerprintMismatch::InputContentChanged { path } => {
                write!(f, "File content changed: {:?}", path)
            }
            FingerprintMismatch::EnvChanged(env_diff) => {
                write!(f, "Environment variables changed: {:?}", env_diff)
            }
        }
    }
}

impl TaskFingerprint {
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
        Ok(Self { config: task.config.clone(), inputs, envs: task.envs.env_fingerprint })
    }

    pub fn validate(
        &self,
        current_config: &TaskConfig,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> anyhow::Result<Option<FingerprintMismatch>> {
        let task_envs = TaskEnvs::resolve(current_config)?;

        // TODO: use diff result instead of eq
        Ok(if &self.config != current_config {
            Some(FingerprintMismatch::ConfigChanged(self.config.diff(current_config)))
        } else if &self.envs != &task_envs.env_fingerprint {
            Some(FingerprintMismatch::EnvChanged(self.envs.diff(&task_envs.env_fingerprint)))
        } else {
            let input_mismatch =
                self.inputs.par_iter().find_map_any(|(input_relative_path, path_fingerprint)| {
                    let input_full_path =
                        Arc::<OsStr>::from(base_dir.join(input_relative_path).into_os_string());
                    let current_path_fingerprint = match fs.fingerprint_path(&input_full_path) {
                        Ok(ok) => ok,
                        Err(err) => return Some(Err(err.into())),
                    };
                    if path_fingerprint != &current_path_fingerprint {
                        Some(anyhow::Ok(FingerprintMismatch::InputContentChanged {
                            path: input_relative_path.clone(),
                        }))
                    } else {
                        None
                    }
                });
            input_mismatch.transpose()?
        })
    }
}
