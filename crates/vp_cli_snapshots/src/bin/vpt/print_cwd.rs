pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = std::env::current_dir()?;
    println!("{}", cwd.display());
    Ok(())
}
