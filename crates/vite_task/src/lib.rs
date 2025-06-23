mod cache;
mod config;
mod execute;
mod fingerprint;
mod fs;
mod schedule;
mod str;

use std::path::PathBuf;

use crate::str::Str;

use crate::{config::Workspace, schedule::ExecutionPlan};

#[derive(Debug)]
pub struct Args {
    pub tasks: Vec<Str>,
}

pub fn main(cwd: PathBuf, args: Args) -> anyhow::Result<()> {
    let mut workspace = Workspace::load(cwd)?;
    let task_graph = workspace.to_task_graph(args.tasks)?;
    let plan = ExecutionPlan::plan(task_graph)?;
    plan.execute(&mut workspace)?;

    workspace.unload()?;
    Ok(())
}
