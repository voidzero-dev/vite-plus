pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut parents = false;
    let mut paths = Vec::new();
    for arg in args {
        match arg.as_str() {
            "-p" => parents = true,
            _ => paths.push(arg.as_str()),
        }
    }
    if paths.is_empty() {
        return Err("Usage: vpt mkdir [-p] <path>...".into());
    }
    for path in paths {
        if parents {
            std::fs::create_dir_all(path)?;
        } else {
            std::fs::create_dir(path)?;
        }
    }
    Ok(())
}
