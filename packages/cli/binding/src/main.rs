use clap::Parser as _;
use vite_error::Error;
use vite_path::current_dir;

mod cli;
mod commands;

use cli::{Args, CliOptions, init_tracing};

#[tokio::main]

async fn main() -> Result<(), Error> {
    init_tracing();

    let args = Args::parse();

    let result = cli::main(current_dir()?, args, None::<CliOptions>).await;

    match result {
        Ok(exit_status) => std::process::exit(exit_status.code().unwrap_or(1)),

        Err(err) => {
            tracing::error!("Error: {}", err);
            match err {
                // Standard exit code for Ctrl+C
                Error::UserCancelled => std::process::exit(130),
                _ => return Err(err),
            }
        }
    }
}
