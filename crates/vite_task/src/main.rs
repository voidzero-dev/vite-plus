use vite_path::current_dir;

use clap::Parser as _;
use vite_task::{Args, CliOptions, init_tracing};

use vite_error::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    init_tracing();

    let args = Args::parse();
    vite_task::main(current_dir()?, args, None::<CliOptions>).await
}
