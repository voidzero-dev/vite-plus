use clap::Parser as _;
use vite_error::Error;
use vite_path::current_dir;
use vite_task::{Args, CliOptions, init_tracing};

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();

    let args = Args::parse();
    let result = vite_task::main(current_dir()?, args, None::<CliOptions>).await;

    match result {
        Ok(Some(exit_status)) => {
            // Exit with the exit status of the first failed task
            std::process::exit(exit_status.code().unwrap_or(1))
        }
        Ok(None) => {
            // Success case - no failed tasks
            Ok(())
        }
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
