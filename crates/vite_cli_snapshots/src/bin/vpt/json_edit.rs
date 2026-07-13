/// json-edit `<file>` `<dot-path>` `<value>`
///
/// Sets a (possibly nested) key in a JSON file. `value` is parsed as JSON;
/// if that fails it is treated as a plain string. Intermediate objects are
/// created as needed.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let [file, dot_path, raw_value] = args else {
        return Err("Usage: vpt json-edit <file> <dot-path> <value>".into());
    };

    let path = std::path::Path::new(file);
    let content = std::fs::read_to_string(path)?;
    let mut root: serde_json::Value = serde_json::from_str(&content)?;
    let new_value: serde_json::Value = serde_json::from_str(raw_value)
        .unwrap_or_else(|_| serde_json::Value::String(raw_value.clone()));

    let segments: Vec<&str> = dot_path.split('.').collect();
    let (last, init) = segments.split_last().ok_or("dot-path must contain at least one segment")?;
    let mut cursor = &mut root;
    for segment in init {
        cursor = cursor
            .as_object_mut()
            .ok_or_else(|| std::format!("segment '{segment}' is not an object"))?
            .entry((*segment).to_owned())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    }
    cursor
        .as_object_mut()
        .ok_or_else(|| std::format!("parent of '{last}' is not an object"))?
        .insert((*last).to_owned(), new_value);

    let mut out = serde_json::to_string_pretty(&root)?;
    out.push('\n');
    std::fs::write(path, out)?;
    Ok(())
}
