mod cache;
mod collections;
mod config;
mod execute;
mod fingerprint;
mod fs;
mod maybe_str;
mod schedule;
mod str;
mod cmd;

use std::iter;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use itertools::Itertools;
use serde::Serialize;

use crate::cache::{CachedTask, TaskCacheKey};
use crate::collections::HashMap;
use crate::config::TaskId;
use crate::str::Str;

use crate::{config::Workspace, schedule::ExecutionPlan};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// A list of tasks to run.
    #[clap(num_args = 0..)]
    pub tasks: Vec<Str>,

    /// Optional arguments for the tasks, captured after '--'.
    #[clap(last = true)]
    pub task_args: Vec<Str>,

    /// Display cache for debugging.
    #[clap(short, long)]
    pub debug: bool,
}

pub async fn main(cwd: PathBuf, args: Args) -> anyhow::Result<()> {
    let mut workspace = Workspace::load(cwd)?;
    let task_args = Arc::<[Str]>::from(args.task_args);
    let task_graph = workspace.resolve_tasks(&args.tasks, task_args.clone())?;
    if args.debug {
        #[derive(Serialize)]
        struct CacheEntry {
            cache_key: TaskCacheKey,
            cached_task: Option<CachedTask>,
        }
        let cache = workspace.cache();
        let mut task_cache_map = Vec::<CacheEntry>::new();
        if args.tasks.is_empty() {
            cache.list_cache(|cache_key, cached_task| {
                task_cache_map.push(CacheEntry { cache_key, cached_task: Some(cached_task) });
                Ok(())
            })?;
        } else {
            for resolved_task in task_graph.node_weights() {
                let cached_task = cache.get_cache(resolved_task.id.clone(), task_args.clone())?;
                task_cache_map.push(CacheEntry {
                    cache_key: TaskCacheKey {
                        task_id: resolved_task.id.clone(),
                        args: task_args.clone(),
                    },
                    cached_task,
                });
            }
        }
        let cache_debug_json = serde_json::to_string_pretty(&task_cache_map)?;
        let _ = edit::edit(&cache_debug_json)?;
    } else {
        let plan = ExecutionPlan::plan(task_graph)?;
        plan.execute(&mut workspace).await?;

        workspace.unload()?;
    }
    Ok(())
}
