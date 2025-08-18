use std::env::current_dir;

use clap::Parser as _;

use vite_error::Error;
use vite_task::{Args, CliOptions, init_tracing};

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();

    let args = Args::parse();
    vite_task::main(current_dir()?, args, None::<CliOptions>).await.map_err(|e| {
        tracing::error!("Error: {}", e);
        e
    })
}
