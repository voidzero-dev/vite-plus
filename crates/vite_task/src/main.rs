use std::env::current_dir;

use clap::Parser as _;

use vite_error::Error;
use vite_task::{Args, CliOptions, init_tracing};

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();

    let args = Args::parse();
    let result = vite_task::main(current_dir()?, args, None::<CliOptions>).await;
    if let Err(err) = result {
        tracing::error!("Error: {}", err);
        match err {
            Error::UserCancelled => std::process::exit(130), // Standard exit code for Ctrl+C
            _ => return Err(err),
        }
    }
    Ok(())
}
