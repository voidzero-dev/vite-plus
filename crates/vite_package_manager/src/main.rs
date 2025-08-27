use vite_error::Error;
use vite_package_manager::package_manager::PackageManager;
use vite_path::current_dir;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let current_dir = current_dir()?;
    let package_manager = PackageManager::builder(&current_dir).build().await?;
    println!("Package manager: {:#?} for {:?}", package_manager, current_dir);

    let resolve_command = package_manager.resolve_command();
    println!("Resolve command: {:#?}", resolve_command);

    Ok(())
}
