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
            Error::UserCancelled(exit_code) => std::process::exit(exit_code),
            _ => return Err(err),
        }
    }
    Ok(())
}
