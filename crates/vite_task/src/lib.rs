mod config;
mod schedule;
mod fingerprint;
mod str;
mod cache;
mod fs;

use std::{fs::File, io::BufReader, path::PathBuf};

use crate::str::Str;

use crate::{
    config::{ViteTaskJson, Workspace},
    schedule::ExecutionPlan,
};

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
