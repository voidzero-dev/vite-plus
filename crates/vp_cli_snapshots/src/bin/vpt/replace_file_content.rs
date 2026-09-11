pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        return Err("Usage: vpt replace-file-content <filename> <searchValue> <newValue>".into());
    }
    let filename = &args[0];
    let search_value = &args[1];
    let new_value = &args[2];

    let filepath = std::path::Path::new(filename).canonicalize()?;
    let content = std::fs::read_to_string(&filepath)?;
    if !content.contains(search_value) {
        return Err(std::format!("searchValue not found in {filename}: {search_value:?}").into());
    }
    let new_content = content.replacen(search_value, new_value, 1);
    std::fs::write(&filepath, new_content)?;
    Ok(())
}
