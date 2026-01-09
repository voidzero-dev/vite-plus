use vite_error::Error;
use vite_path::current_dir;

mod cli;

use cli::init_tracing;

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();

    // Pass None for args - main.rs uses env::args() directly
    let result = cli::main(current_dir()?, None, None).await;

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
