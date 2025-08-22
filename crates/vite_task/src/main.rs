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
        if matches!(err, Error::UserCancelled(_)) {
            std::process::exit(130);
        }
    }
    Ok(())
}
