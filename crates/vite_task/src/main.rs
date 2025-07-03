use std::env::current_dir;

use clap::Parser as _;
use vite_task::Args;

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    vite_task::main(current_dir()?, args)
}
