pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (recursive, paths) = match args {
        [flag, src, dst] if flag == "-r" => (true, [src.as_str(), dst.as_str()]),
        [src, dst] => (false, [src.as_str(), dst.as_str()]),
        _ => return Err("Usage: vpt cp [-r] <src> <dst>".into()),
    };

    let src = std::path::Path::new(paths[0]);
    let dst = std::path::Path::new(paths[1]);
    // Copying INTO an existing directory nests under the source name, like
    // real cp (both the file and -r forms).
    let target = if dst.is_dir() {
        dst.join(src.file_name().ok_or("source has no file name")?)
    } else {
        dst.to_path_buf()
    };
    if src.is_dir() {
        if !recursive {
            return Err("copying a directory requires -r".into());
        }
        cp_r::CopyOptions::new().copy_tree(src, &target)?;
    } else {
        std::fs::copy(src, &target)?;
    }
    Ok(())
}
