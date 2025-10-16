use color_eyre::Result;
use vite_tui::{App, logging};

#[tokio::main]
async fn main() -> Result<()> {
    logging::init()?;

    let mut app = App::new();
    app.run().await?;
    Ok(())
}
