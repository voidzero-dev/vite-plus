use vite_error::Error;
use vite_package_manager::package_manager::detect_package_manager;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let current_dir = std::env::current_dir()?;
    let package_manager = detect_package_manager(&current_dir).await?;
    println!("Package manager: {:#?} for {:?}", package_manager, current_dir);

    Ok(())
}
