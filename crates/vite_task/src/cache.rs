use std::path::Path;
use std::sync::{Arc, Mutex};

// use bincode::config::{Configuration, standard};
use bincode::{Decode, Encode, decode_from_slice, encode_to_vec};
use rusqlite::{Connection, OptionalExtension as _};
use serde::Serialize;

use crate::config::ResolvedTask;
use crate::execute::{ExecutedTask, StdOutput};
use crate::fingerprint::{FingerprintMismatch, TaskFingerprint};
use crate::fs::FileSystem;
use crate::str::Str;

#[derive(Debug, Encode, Decode, Serialize)]
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
    conn: Mutex<Connection>,
}

#[derive(Debug, Hash, Encode, Decode, Serialize)]
pub struct TaskCacheKey {
    pub task_name: Str,
    pub args: Arc<[Str]>,
}

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug)]
pub enum CacheMiss {
    NotFound,
    FingerprintMismatch(FingerprintMismatch),
}

impl TaskCache {
    pub fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL")?;
        conn.execute("BEGIN EXCLUSIVE", ())?;
        loop {
            let user_version: u32 = conn.query_one("PRAGMA user_version", (), |row| row.get(0))?;
            match user_version {
                0 => {
                    // fresh new db
                    conn.execute("CREATE TABLE tasks (key BLOB PRIMARY KEY, value BLOB);", ())?;
                    conn.execute("PRAGMA user_version = 1", ())?;
                }
                // Migration done here
                1 => break,
                2.. => anyhow::bail!("Unrecognized cache db version: {user_version}"),
            }
        }
        Ok(Self { conn: Mutex::new(conn) })
    }
    pub fn save(self) -> anyhow::Result<()> {
        let conn = self.conn.into_inner().unwrap();
        conn.execute("COMMIT", ())?;
        Ok(())
    }

    pub fn update(
        &mut self,
        task_name: Str,
        args: Arc<[Str]>,
        cached_task: CachedTask,
    ) -> anyhow::Result<()> {
        let key = TaskCacheKey { task_name, args };
        let conn = self.conn.lock().unwrap();
        let key_blob = encode_to_vec(&key, BINCODE_CONFIG)?;
        let value_blob = encode_to_vec(&cached_task, BINCODE_CONFIG)?;
        let mut update_stmt = conn.prepare_cached(
            "INSERT INTO tasks (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2"
        )?;
        update_stmt.execute([key_blob, value_blob])?;
        Ok(())
    }

    pub fn get_cache(
        &self,
        task_name: Str,
        args: Arc<[Str]>,
    ) -> anyhow::Result<Option<CachedTask>> {
        let conn = self.conn.lock().unwrap();
        let mut select_stmt = conn.prepare_cached("SELECT value FROM tasks WHERE key=?")?;
        let key_blob = encode_to_vec(&TaskCacheKey { task_name, args }, BINCODE_CONFIG)?;
        let Some(value_blob) =
            select_stmt.query_row::<Vec<u8>, _, _>([key_blob], |row| row.get(0)).optional()?
        else {
            return Ok(None);
        };
        let (cached_task, _) = decode_from_slice::<CachedTask, _>(&value_blob, BINCODE_CONFIG)?;
        Ok(Some(cached_task))
    }

    pub fn list_cache(
        &self,
        mut f: impl FnMut(TaskCacheKey, CachedTask) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        let mut select_stmt = conn.prepare_cached("SELECT key, value FROM tasks")?;
        let cache_list = select_stmt.query_and_then((), |row| {
            let key_blob: Vec<u8> = row.get(0)?;
            let value_blob: Vec<u8> = row.get(1)?;
            let (key, _) = decode_from_slice::<TaskCacheKey, _>(&key_blob, BINCODE_CONFIG)?;
            let (cached_task, _) = decode_from_slice::<CachedTask, _>(&value_blob, BINCODE_CONFIG)?;
            anyhow::Ok((key, cached_task))
        })?;
        for cache in cache_list {
            let (key, cached_task) = cache?;
            f(key, cached_task)?;
        }
        Ok(())
    }

    /// Tries to get the task cache if the fingerprint matches, otherwise returns why the cache misses
    pub fn try_hit(
        &self,
        task: &ResolvedTask,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> anyhow::Result<Result<CachedTask, CacheMiss>> {
        let Some(cached_task) = self.get_cache(task.name.clone(), task.args.clone())? else {
            return Ok(Err(CacheMiss::NotFound));
        };
        if let Some(fingerprint_mismatch) = cached_task.fingerprint.validate(task, fs, base_dir)? {
            return Ok(Err(CacheMiss::FingerprintMismatch(fingerprint_mismatch)));
        }
        Ok(Ok(cached_task))
    }
}
