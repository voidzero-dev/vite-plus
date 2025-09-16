use clap::Parser as _;
use vite_error::Error;
use vite_path::current_dir;
use vite_task::{Args, CliOptions, init_tracing};

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();

    let args = Args::parse();
    match vite_task::main(current_dir()?, args, None::<CliOptions>).await {
        Ok(()) => Ok(()),
        Err(err) => {
            tracing::error!("Error: {}", err);
            match err {
                Error::UserCancelled => std::process::exit(130), // Standard exit code for Ctrl+C
                _ => return Err(err),
            }
        }
    }
}
