use std::{collections::HashMap, ffi::OsStr, fmt::Display, path::Path, sync::Arc};

use crate::{config::TaskConfig, execute::ExecutedTask, fs::FileSystem, str::Str};
use bincode::{Decode, Encode};
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
    ConfigChanged { old_config: TaskConfig, new_config: TaskConfig },
    InputContentChanged { path: Str },
    EnvChanged { name: Str, old_value: Option<Str>, new_value: Option<Str> },
}

impl Display for FingerprintMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FingerprintMismatch::ConfigChanged { old_config, new_config } => {
                write!(f, "Config inputs changed: {:?} => {:?}", old_config, new_config)
            }
            FingerprintMismatch::InputContentChanged { path } => {
                write!(f, "File content changed: {:?}", path)
            }
            FingerprintMismatch::EnvChanged { name, old_value, new_value } => {
                write!(
                    f,
                    "Environment variable '{}' changed: {:?} => {:?}",
                    name, old_value, new_value
                )
            }
        }
    }
}

impl TaskFingerprint {
    pub fn create(
        task_config: &TaskConfig,
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
        Ok(Self { config: task_config.clone(), inputs, envs: executed_task.envs.clone() })
    }
    pub fn validate(
        &self,
        task: &TaskConfig,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> anyhow::Result<Option<FingerprintMismatch>> {
        if &self.config != task {
            return Ok(Some(FingerprintMismatch::ConfigChanged {
                old_config: self.config.clone(),
                new_config: task.clone(),
            }));
        }
        let input_mismatch =
            self.inputs.par_iter().find_map_any(|(input_relative_path, path_fingerprint)| {
                let input_full_path =
                    Arc::<OsStr>::from(base_dir.join(input_relative_path).into_os_string());
                let current_path_fingerprint = match fs.fingerprint_path(&input_full_path) {
                    Ok(ok) => ok,
                    Err(err) => return Some(Err(err)),
                };
                if path_fingerprint != &current_path_fingerprint {
                    Some(Ok(FingerprintMismatch::InputContentChanged {
                        path: input_relative_path.clone(),
                    }))
                } else {
                    None
                }
            });
        if let Some(input_mismatch) = input_mismatch.transpose()? {
            return Ok(Some(input_mismatch));
        }
        Ok(None)
    }
}
