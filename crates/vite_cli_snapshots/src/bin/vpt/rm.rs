pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut recursive = false;
    let mut force = false;
    let mut paths = Vec::new();
    for arg in args {
        match arg.as_str() {
            "-r" => recursive = true,
            "-f" => force = true,
            "-rf" | "-fr" => {
                recursive = true;
                force = true;
            }
            _ => paths.push(arg.as_str()),
        }
    }
    if paths.is_empty() {
        return Err("Usage: vpt rm [-rf] <path>...".into());
    }
    for path in paths {
        let p = std::path::Path::new(path);
        let result = if p.is_dir() && recursive {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        };
        // `-f` keeps rm semantics: a missing target is not an error, so
        // cleanup of optional artifacts stays deterministic.
        if let Err(err) = result {
            if !(force && err.kind() == std::io::ErrorKind::NotFound) {
                return Err(err.into());
            }
        }
    }
    Ok(())
}
