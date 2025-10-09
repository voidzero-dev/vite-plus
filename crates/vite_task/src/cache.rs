use std::{fmt::Display, io::Write, sync::Arc, time::Duration};

// use bincode::config::{Configuration, standard};
use bincode::{Decode, Encode, decode_from_slice, encode_to_vec};
use rusqlite::{Connection, OptionalExtension as _, config::DbConfig};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use vite_path::{AbsolutePath, AbsolutePathBuf};
use vite_str::Str;

use crate::{
    Error,
    config::{CommandFingerprint, ResolvedTask, TaskId},
    execute::{ExecutedTask, StdOutput},
    fingerprint::{PostRunFingerprint, PostRunFingerprintMismatch},
    fs::FileSystem,
};

/// Command cache value, for validating post-run fingerprint after the command fingerprint is matched,
/// and replaying the std outputs if validated.
#[derive(Debug, Encode, Decode, Serialize)]
pub struct CommandCacheValue {
    pub post_run_fingerprint: PostRunFingerprint,
    pub std_outputs: Arc<[StdOutput]>,
    pub duration: Duration,
}

impl CommandCacheValue {
    pub fn create(
        executed_task: ExecutedTask,
        fs: &impl FileSystem,
        base_dir: &AbsolutePath,
        fingerprint_ignores: Option<&[Str]>,
    ) -> Result<Self, Error> {
        let post_run_fingerprint =
            PostRunFingerprint::create(&executed_task, fs, base_dir, fingerprint_ignores)?;
        Ok(Self {
            post_run_fingerprint,
            std_outputs: executed_task.std_outputs,
            duration: executed_task.duration,
        })
    }
}

#[derive(Debug)]
pub struct TaskCache {
    conn: Mutex<Connection>,
    pub(crate) path: AbsolutePathBuf,
}

/// Key to identify a task run.
/// It includes the additional args, so the same task with different args wouldn't overwrite each other.
#[derive(Debug, Encode, Decode, Serialize)]
pub struct TaskRunKey {
    pub task_id: TaskId,
    pub args: Arc<[Str]>,
}

const BINCODE_CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CacheMiss {
    NotFound,
    FingerprintMismatch(FingerprintMismatch),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FingerprintMismatch {
    /// Found the cache entry of the same task run, but the command fingerprint mismatches
    /// this happens when the command itself or an env changes.
    CommandFingerprintMismatch(CommandFingerprint),
    /// Found the cache entry with the same command fingerprint, but the post-run fingerprint mismatches
    PostRunFingerprintMismatch(PostRunFingerprintMismatch),
}

impl Display for FingerprintMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandFingerprintMismatch(diff) => {
                // TODO: improve the display of command fingerprint diff
                write!(f, "Command fingerprint changed: {diff:?}")
            }
            Self::PostRunFingerprintMismatch(diff) => Display::fmt(diff, f),
        }
    }
}

impl TaskCache {
    pub fn load_from_path(cache_path: AbsolutePathBuf) -> Result<Self, Error> {
        let path: &AbsolutePath = cache_path.as_ref();
        tracing::info!("Creating task cache directory at {:?}", path);
        std::fs::create_dir_all(path)?;

        let db_path = path.join("cache.db");
        let conn = Connection::open(db_path.as_path())?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        loop {
            let user_version: u32 = conn.query_one("PRAGMA user_version", (), |row| row.get(0))?;
            match user_version {
                0 => {
                    // fresh new db
                    conn.execute(
                        "CREATE TABLE command_cache (key BLOB PRIMARY KEY, value BLOB);",
                        (),
                    )?;
                    conn.execute(
                        "CREATE TABLE taskrun_to_command (key BLOB PRIMARY KEY, value BLOB);",
                        (),
                    )?;
                    conn.execute("PRAGMA user_version = 3", ())?;
                }
                1..=2 => {
                    // old internal db version. reset
                    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, true)?;
                    conn.execute("VACUUM", ())?;
                    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, false)?;
                }
                3 => break, // current version
                4.. => return Err(Error::UnrecognizedDbVersion(user_version)),
            }
        }
        Ok(Self { conn: Mutex::new(conn), path: cache_path })
    }

    #[tracing::instrument]
    pub async fn save(self) -> Result<(), Error> {
        // do some cleanup in the future
        Ok(())
    }

    pub async fn update(
        &self,
        resolved_task: &ResolvedTask,
        cached_task: CommandCacheValue,
    ) -> Result<(), Error> {
        let task_run_key =
            TaskRunKey { task_id: resolved_task.id(), args: resolved_task.args.clone() };
        let command_fingerprint = &resolved_task.resolved_command.fingerprint;
        self.upsert_command_cache(command_fingerprint, &cached_task).await?;
        self.upsert_taskrun_to_command(&task_run_key, command_fingerprint).await?;
        Ok(())
    }

    /// Tries to get the task cache if the fingerprint matches, otherwise returns why the cache misses
    pub async fn try_hit(
        &self,
        task: &ResolvedTask,
        fs: &impl FileSystem,
        base_dir: &AbsolutePath,
    ) -> Result<Result<CommandCacheValue, CacheMiss>, Error> {
        let task_run_key = TaskRunKey { task_id: task.id(), args: task.args.clone() };
        let command_fingerprint = &task.resolved_command.fingerprint;
        // Try to directly find the command cache by command fingerprint first, ignoring the task run key
        if let Some(cache_value) =
            self.get_command_cache_by_command_fingerprint(command_fingerprint).await?
        {
            if let Some(post_run_fingerprint_mismatch) =
                cache_value.post_run_fingerprint.validate(fs, base_dir)?
            {
                // Found the command cache with the same command fingerprint, but the post-run fingerprint mismatches
                Ok(Err(CacheMiss::FingerprintMismatch(
                    FingerprintMismatch::PostRunFingerprintMismatch(post_run_fingerprint_mismatch),
                )))
            } else {
                // Associate the task run key to the command fingerprint if not already,
                // so that next time we can find it and report command fingerprint mismatch
                self.upsert_taskrun_to_command(&task_run_key, command_fingerprint).await?;
                Ok(Ok(cache_value))
            }
        } else if let Some(command_fingerprint_in_cache) =
            self.get_command_fingerprint_by_task_run_key(&task_run_key).await?
        {
            // No command cache found with the current command fingerprint,
            // but found a command fingerprint associated with the same task run key,
            // meaning the command or env has changed since last run
            Ok(Err(CacheMiss::FingerprintMismatch(
                FingerprintMismatch::CommandFingerprintMismatch(command_fingerprint_in_cache),
            )))
        } else {
            Ok(Err(CacheMiss::NotFound))
        }
    }
}

// basic database operations
impl TaskCache {
    async fn get_key_by_value<K: Encode, V: Decode<()>>(
        &self,
        table: &str,
        key: &K,
    ) -> Result<Option<V>, Error> {
        let conn = self.conn.lock().await;
        let mut select_stmt =
            conn.prepare_cached(&format!("SELECT value FROM {table} WHERE key=?"))?;
        let key_blob = encode_to_vec(key, BINCODE_CONFIG)?;
        let Some(value_blob) =
            select_stmt.query_row::<Vec<u8>, _, _>([key_blob], |row| row.get(0)).optional()?
        else {
            return Ok(None);
        };
        let (value, _) = decode_from_slice::<V, _>(&value_blob, BINCODE_CONFIG)?;
        Ok(Some(value))
    }

    async fn get_command_cache_by_command_fingerprint(
        &self,
        command_fingerprint: &CommandFingerprint,
    ) -> Result<Option<CommandCacheValue>, Error> {
        self.get_key_by_value("command_cache", command_fingerprint).await
    }

    async fn get_command_fingerprint_by_task_run_key(
        &self,
        task_run_key: &TaskRunKey,
    ) -> Result<Option<CommandFingerprint>, Error> {
        self.get_key_by_value("taskrun_to_command", task_run_key).await
    }

    async fn upsert<K: Encode, V: Encode>(
        &self,
        table: &str,
        key: &K,
        value: &V,
    ) -> Result<(), Error> {
        let conn = self.conn.lock().await;
        let key_blob = encode_to_vec(key, BINCODE_CONFIG)?;
        let value_blob = encode_to_vec(value, BINCODE_CONFIG)?;
        let mut update_stmt = conn.prepare_cached(&format!(
            "INSERT INTO {table} (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=?2"
        ))?;
        update_stmt.execute([key_blob, value_blob])?;
        Ok(())
    }

    async fn upsert_command_cache(
        &self,
        command_fingerprint: &CommandFingerprint,
        cached_task: &CommandCacheValue,
    ) -> Result<(), Error> {
        self.upsert("command_cache", command_fingerprint, cached_task).await
    }

    async fn upsert_taskrun_to_command(
        &self,
        task_run_key: &TaskRunKey,
        command_fingerprint: &CommandFingerprint,
    ) -> Result<(), Error> {
        self.upsert("taskrun_to_command", task_run_key, command_fingerprint).await
    }

    async fn list_table<K: Decode<()> + Serialize, V: Decode<()> + Serialize>(
        &self,
        table: &str,
        out: &mut impl Write,
    ) -> Result<(), Error> {
        let conn = self.conn.lock().await;
        let mut select_stmt = conn.prepare_cached(&format!("SELECT key, value FROM {table}"))?;
        let mut rows = select_stmt.query([])?;
        while let Some(row) = rows.next()? {
            let key_blob: Vec<u8> = row.get(0)?;
            let value_blob: Vec<u8> = row.get(1)?;
            let (key, _) = decode_from_slice::<K, _>(&key_blob, BINCODE_CONFIG)?;
            let (value, _) = decode_from_slice::<V, _>(&value_blob, BINCODE_CONFIG)?;
            writeln!(
                out,
                "{} => {}",
                serde_json::to_string_pretty(&key)?,
                serde_json::to_string_pretty(&value)?
            )?;
        }
        Ok(())
    }

    pub async fn list(&self, mut out: impl Write) -> Result<(), Error> {
        out.write_all(b"------- taskrun_to_command -------\n")?;
        self.list_table::<TaskRunKey, CommandFingerprint>("taskrun_to_command", &mut out).await?;
        out.write_all(b"------- command_cache -------\n")?;
        self.list_table::<CommandFingerprint, CommandCacheValue>("command_cache", &mut out).await?;
        Ok(())
    }
}
