use rusqlite::config::DbConfig;
use std::fmt::Display;
use std::sync::Arc;
use vite_path::AbsolutePath;

// use bincode::config::{Configuration, standard};
use bincode::{Decode, Encode, de, decode_from_slice, encode_to_vec};
use rusqlite::{Connection, OptionalExtension as _};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::Error;
use crate::config::{CommandFingerprint, CommandFingerprintDiff, ResolvedTask, TaskId};
use crate::execute::{ExecutedTask, StdOutput};
use crate::fingerprint::{PostRunFingerprint, PostRunFingerprintMismatch};
use crate::fs::FileSystem;
use vite_str::Str;

/// Command cache value, for validating post-run fingerprint after the command fingerprint is matched,
/// and replaying the std outputs if validated.
#[derive(Debug, Encode, Decode, Serialize)]
pub struct CommandCacheValue {
    pub post_run_fingerprint: PostRunFingerprint,
    pub std_outputs: Arc<[StdOutput]>,
}

impl CommandCacheValue {
    pub fn create(
        executed_task: ExecutedTask,
        fs: &impl FileSystem,
        base_dir: &AbsolutePath,
    ) -> Result<Self, Error> {
        let post_run_fingerprint = PostRunFingerprint::create(&executed_task, fs, base_dir)?;
        Ok(Self { post_run_fingerprint, std_outputs: executed_task.std_outputs })
    }
}

#[derive(Debug)]
pub struct TaskCache {
    conn: Mutex<Connection>,
}

/// Key to identify a task run.
/// It includes the additional args, so the same task with different args wouldn't overwrite each other.
#[derive(Debug, Encode, Decode, Serialize)]
pub struct TaskRunKey {
    pub task_id: TaskId,
    pub args: Arc<[Str]>,
}

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug)]
pub enum CacheMiss {
    NotFound,
    FingerprintMismatch(FingerprintMismatch),
}

#[derive(Debug)]
pub enum FingerprintMismatch {
    /// Found the cache entry of the same task run, but the command fingerprint mismatches
    /// this happens when the command itself or an env changes.
    CommandFingerprintMismatch(CommandFingerprintDiff),
    /// Found the cache entry with the same command fingerprint, but the post-run fingerprint mismatches
    PostRunFingerprintMismatch(PostRunFingerprintMismatch),
}

impl Display for FingerprintMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FingerprintMismatch::CommandFingerprintMismatch(diff) => {
                // TODO: improve the display of command fingerprint diff
                write!(f, "Command fingerprint changed: {:?}", diff)
            }
            FingerprintMismatch::PostRunFingerprintMismatch(diff) => Display::fmt(diff, f),
        }
    }
}

impl TaskCache {
    pub fn load_from_file(path: impl AsRef<AbsolutePath>) -> Result<Self, Error> {
        let path = path.as_ref();
        let conn = Connection::open(path.as_path())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; BEGIN EXCLUSIVE;")?;
        loop {
            let user_version: u32 = conn.query_one("PRAGMA user_version", (), |row| row.get(0))?;
            match user_version {
                0 => {
                    // fresh new db
                    conn.execute("CREATE TABLE command_cache (command_fingerprint BLOB PRIMARY KEY, value BLOB);", ())?;
                    conn.execute(
                        "CREATE TABLE task_to_command (task_key BLOB PRIMARY KEY, command_fingerprint BLOB);",
                        (),
                    )?;
                    conn.execute("PRAGMA user_version = 2", ())?;
                }
                1 => {
                    // internal versions during dev, we just rebuild the whole cache
                    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, true)?;
                    conn.execute("VACUUM", ())?;
                    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, false)?;
                }
                2 => break, // current version
                3.. => return Err(Error::UnrecognizedDbVersion(user_version)),
            }
        }
        conn.execute_batch("COMMIT")?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    #[tracing::instrument]
    pub async fn save(self) -> Result<(), Error> {
        // do some cleanup in the future
        Ok(())
    }

    fn get_command_cache(
        &self,
        command_fingerprint: &CommandFingerprint,
    ) -> Result<Option<CommandCacheValue>, Error> {
        let conn = self.conn.blocking_lock();
        let mut select_stmt =
            conn.prepare_cached("SELECT value FROM command_cache WHERE command_fingerprint=?")?;
        let key_blob = encode_to_vec(command_fingerprint, BINCODE_CONFIG)?;
        let Some(value_blob) =
            select_stmt.query_row::<Vec<u8>, _, _>([key_blob], |row| row.get(0)).optional()?
        else {
            return Ok(None);
        };
        let (cached_task, _) =
            decode_from_slice::<CommandCacheValue, _>(&value_blob, BINCODE_CONFIG)?;
        Ok(Some(cached_task))
    }

    pub async fn update(
        &mut self,
        resolved_task: &ResolvedTask,
        cached_task: CommandCacheValue,
    ) -> Result<(), Error> {
        todo!()
        // let key = TaskCacheKey {
        //     command_fingerprint: resolved_task.resolved_command.fingerprint.clone(),
        //     args: resolved_task.args.clone(),
        // };
        // let conn = self.conn.lock().await;
        // let key_blob = encode_to_vec(&key, BINCODE_CONFIG)?;
        // let value_blob = encode_to_vec(&cached_task, BINCODE_CONFIG)?;
        // let mut update_stmt = conn.prepare_cached(
        //     "INSERT INTO tasks (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2"
        // )?;
        // update_stmt.execute([key_blob, value_blob])?;
        // Ok(())
    }

    pub async fn get_cache(
        &self,
        resolved_task: &ResolvedTask,
    ) -> Result<Option<CommandCacheValue>, Error> {
        todo!()
        // let key = TaskCacheKey {
        //     command_fingerprint: resolved_task.resolved_command.fingerprint.clone(),
        //     args: resolved_task.args.clone(),
        // };
        // let conn = self.conn.lock().await;
        // let mut select_stmt = conn.prepare_cached("SELECT value FROM tasks WHERE key=?")?;
        // let key_blob = encode_to_vec(&key, BINCODE_CONFIG)?;
        // let Some(value_blob) =
        //     select_stmt.query_row::<Vec<u8>, _, _>([key_blob], |row| row.get(0)).optional()?
        // else {
        //     return Ok(None);
        // };
        // let (cached_task, _) = decode_from_slice::<CommandCacheValue, _>(&value_blob, BINCODE_CONFIG)?;
        // Ok(Some(cached_task))
    }

    // pub async fn list_cache(
    //     &self,
    //     mut f: impl FnMut(TaskCacheKey, CommandCacheValue) -> Result<(), Error>,
    // ) -> Result<(), Error> {
    //     let conn = self.conn.lock().await;
    //     let mut select_stmt = conn.prepare_cached("SELECT key, value FROM tasks")?;
    //     let cache_list = select_stmt.query_and_then((), |row| {
    //         let key_blob: Vec<u8> = row.get(0)?;
    //         let value_blob: Vec<u8> = row.get(1)?;
    //         let (key, _) = decode_from_slice::<TaskCacheKey, _>(&key_blob, BINCODE_CONFIG)?;
    //         let (cached_task, _) = decode_from_slice::<CommandCacheValue, _>(&value_blob, BINCODE_CONFIG)?;
    //         Ok::<_, Error>((key, cached_task))
    //     })?;
    //     for cache in cache_list {
    //         let (key, cached_task) = cache?;
    //         f(key, cached_task)?;
    //     }
    //     Ok(())
    // }

    /// Tries to get the task cache if the fingerprint matches, otherwise returns why the cache misses
    pub async fn try_hit(
        &self,
        task: &ResolvedTask,
        fs: &impl FileSystem,
        base_dir: &AbsolutePath,
    ) -> Result<Result<CommandCacheValue, CacheMiss>, Error> {
        todo!()
        // let Some(cached_task) = self.get_cache(task).await? else {
        //     return Ok(Err(CacheMiss::NotFound));
        // };
        // if let Some(fingerprint_mismatch) = cached_task.post_run_fingerprint.validate(task, fs, base_dir)? {
        //     return Ok(Err(CacheMiss::PostRunFingerprintMismatch(fingerprint_mismatch)));
        // }
        // Ok(Ok(cached_task))
    }
}
