use std::env::current_dir;

use clap::Parser as _;
use vite_task::Args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    vite_task::main(current_dir()?, args).await
}
