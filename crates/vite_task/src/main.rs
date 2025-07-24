use std::env::current_dir;

use clap::Parser as _;
use vite_task::Args;

use vite_error::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    vite_task::main(current_dir()?, args).await
}
