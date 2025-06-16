use std::{
    fs::File,
    io::BufReader,
    path::PathBuf,
};

use compact_str::CompactString;

use crate::config::ViteTaskJson;
mod config;

#[derive(Debug)]
pub struct Args {
    pub tasks: Vec<CompactString>,
}

pub fn main(cwd: PathBuf, args: Args) -> anyhow::Result<()> {
    let config_path = cwd.join("vite-task.json");
    let config: ViteTaskJson = serde_json::from_reader(BufReader::new(File::open(config_path)?))?;
    let task_graph = config.to_task_graph(&cwd, args.tasks)?;
    dbg!(task_graph);
    Ok(())
}
