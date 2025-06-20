use std::{
    env::{args, current_dir},
    path::PathBuf,
};

use vite_task::Args;

fn main() -> anyhow::Result<()> {
    vite_task::main(
        current_dir()?,
        Args { tasks: args().skip(1).map(|arg| arg.as_str().into()).collect() },
    )
}
