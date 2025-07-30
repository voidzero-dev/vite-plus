use std::path::Path;
use std::sync::Arc;

// use bincode::config::{Configuration, standard};
use bincode::{Decode, Encode, decode_from_slice, encode_to_vec};
use rusqlite::{Connection, OptionalExtension as _};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::Error;
use crate::config::{ResolvedTask, TaskId};
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
    ) -> Result<Self, Error> {
        let fingerprint = TaskFingerprint::create(task, &executed_task, fs, base_dir)?;
        Ok(Self { fingerprint, std_outputs: executed_task.std_outputs })
    }
}

#[derive(Debug)]
pub struct TaskCache {
    conn: Mutex<Connection>,
}

#[derive(Debug, Hash, Encode, Decode, Serialize)]
pub struct TaskCacheKey {
    pub task_id: TaskId,
    pub args: Arc<[Str]>,
}

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug)]
pub enum CacheMiss {
    NotFound,
    FingerprintMismatch(FingerprintMismatch),
}

impl TaskCache {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
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
                2.. => return Err(Error::UnrecognizedDbVersion(user_version)),
            }
        }
        Ok(Self { conn: Mutex::new(conn) })
    }

    #[tracing::instrument]
    pub async fn save(self) -> Result<(), Error> {
        let conn = self.conn.lock().await;
        conn.execute("COMMIT", ())?;
        Ok(())
    }

    pub async fn update(
        &mut self,
        task_id: TaskId,
        args: Arc<[Str]>,
        cached_task: CachedTask,
    ) -> Result<(), Error> {
        let key = TaskCacheKey { task_id, args };
        let conn = self.conn.lock().await;
        let key_blob = encode_to_vec(&key, BINCODE_CONFIG)?;
        let value_blob = encode_to_vec(&cached_task, BINCODE_CONFIG)?;
        let mut update_stmt = conn.prepare_cached(
            "INSERT INTO tasks (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2"
        )?;
        update_stmt.execute([key_blob, value_blob])?;
        Ok(())
    }

    pub async fn get_cache(
        &self,
        task_id: TaskId,
        args: Arc<[Str]>,
    ) -> Result<Option<CachedTask>, Error> {
        let conn = self.conn.lock().await;
        let mut select_stmt = conn.prepare_cached("SELECT value FROM tasks WHERE key=?")?;
        let key_blob = encode_to_vec(&TaskCacheKey { task_id, args }, BINCODE_CONFIG)?;
        let Some(value_blob) =
            select_stmt.query_row::<Vec<u8>, _, _>([key_blob], |row| row.get(0)).optional()?
        else {
            return Ok(None);
        };
        let (cached_task, _) = decode_from_slice::<CachedTask, _>(&value_blob, BINCODE_CONFIG)?;
        Ok(Some(cached_task))
    }

    pub async fn list_cache(
        &self,
        mut f: impl FnMut(TaskCacheKey, CachedTask) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let conn = self.conn.lock().await;
        let mut select_stmt = conn.prepare_cached("SELECT key, value FROM tasks")?;
        let cache_list = select_stmt.query_and_then((), |row| {
            let key_blob: Vec<u8> = row.get(0)?;
            let value_blob: Vec<u8> = row.get(1)?;
            let (key, _) = decode_from_slice::<TaskCacheKey, _>(&key_blob, BINCODE_CONFIG)?;
            let (cached_task, _) = decode_from_slice::<CachedTask, _>(&value_blob, BINCODE_CONFIG)?;
            Ok::<_, Error>((key, cached_task))
        })?;
        for cache in cache_list {
            let (key, cached_task) = cache?;
            f(key, cached_task)?;
        }
        Ok(())
    }

    /// Tries to get the task cache if the fingerprint matches, otherwise returns why the cache misses
    pub async fn try_hit(
        &self,
        task: &ResolvedTask,
        fs: &impl FileSystem,
        base_dir: &Path,
    ) -> Result<Result<CachedTask, CacheMiss>, Error> {
        let Some(cached_task) = self.get_cache(task.id.clone(), task.args.clone()).await? else {
            return Ok(Err(CacheMiss::NotFound));
        };
        if let Some(fingerprint_mismatch) = cached_task.fingerprint.validate(task, fs, base_dir)? {
            return Ok(Err(CacheMiss::FingerprintMismatch(fingerprint_mismatch)));
        }
        Ok(Ok(cached_task))
    }
}
