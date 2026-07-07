pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 2 {
        return Err("Usage: vpt write-file <filename> <content>".into());
    }
    let path = std::path::Path::new(&args[0]);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, &args[1])?;
    Ok(())
}
