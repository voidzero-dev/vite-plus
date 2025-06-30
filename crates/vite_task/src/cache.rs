use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// use bincode::config::{Configuration, standard};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::config::{ResolvedTask};
use crate::execute::{ExecutedTask, StdOutput};
use crate::fingerprint::{FingerprintMismatch, TaskFingerprint};
use crate::fs::FileSystem;
use crate::str::Str;

#[derive(Debug, Encode, Decode, Serialize, Deserialize)]
pub struct CachedTask {
    pub fingerprint: TaskFingerprint,
    pub std_outputs: Arc<[StdOutput]>,
}

impl CachedTask {
    pub fn create(
        task: ResolvedTask,
        executed_task: ExecutedTask,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> anyhow::Result<Self> {
        let fingerprint = TaskFingerprint::create(task, &executed_task, fs, base_dir)?;
        Ok(Self { fingerprint, std_outputs: executed_task.std_outputs })
    }
}

pub struct TaskCache {
    cached_tasks_by_name: HashMap<Str, CachedTask>,
    path: PathBuf,
}

// const BINCODE_CONFIG: Configuration = standard();

#[derive(Debug)]
pub enum CacheMiss {
    NotFound,
    FingerprintMismatch(FingerprintMismatch),
}

impl TaskCache {
    pub fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let cached_tasks_by_name: HashMap<Str, CachedTask> = match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                // Using json for easy debugging
                // Will switch to bincode for better performance 
                serde_json::from_reader(reader)?
                // bincode::decode_from_std_read(&mut reader, BINCODE_CONFIG)?
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    HashMap::new()
                } else {
                    return Err(err.into());
                }
            }
        };
        Ok(Self { cached_tasks_by_name, path: path.to_path_buf() })
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let path = self.path.as_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &self.cached_tasks_by_name)?;
        // bincode::encode_into_std_write(&self.cached_tasks_by_name, &mut writer, BINCODE_CONFIG)?;
        writer.into_inner()?.sync_data()?;
        Ok(())
    }

    pub fn update(&mut self, task_name: Str, cached_task: CachedTask) -> anyhow::Result<()> {
        self.cached_tasks_by_name.insert(task_name, cached_task);
        Ok(())
    }

    /// Tries to get the task cache if the fingerprint matches, otherwise returns why the cache misses
    pub fn try_hit<'me>(
        &'me self,
        task: &ResolvedTask,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> anyhow::Result<Result<&'me CachedTask, CacheMiss>> {
        let Some(cached_task) = self.cached_tasks_by_name.get(&task.name) else {
            return Ok(Err(CacheMiss::NotFound));
        };
        if let Some(fingerprint_mismatch) =
            cached_task.fingerprint.validate(&task.config, fs, base_dir)?
        {
            return Ok(Err(CacheMiss::FingerprintMismatch(fingerprint_mismatch)));
        }
        Ok(Ok(cached_task))
    }
}
