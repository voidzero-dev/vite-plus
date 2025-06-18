use std::{
    fs::File,
    io::BufReader,
    path::PathBuf,
};

use compact_str::CompactString;

use crate::config::{ViteTaskJson, Workspace};
mod config;

#[derive(Debug)]
pub struct Args {
    pub tasks: Vec<CompactString>,
}

pub fn main(cwd: PathBuf, args: Args) -> anyhow::Result<()> {
    let workspace = Workspace::load(cwd)?;
    let task_graph = workspace.to_task_graph(args.tasks)?;
    dbg!(task_graph);
    Ok(())
}
