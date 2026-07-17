/// grep-file `<path>` `<pattern>`
///
/// Prints found/missing for the snapshot and keeps grep's exit semantics:
/// nonzero when the pattern is absent or the file is unreadable, so
/// content guards short-circuit under the runner's failure flow.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let [path, pattern] = args else {
        return Err("Usage: vpt grep-file <path> <pattern>".into());
    };
    match std::fs::read_to_string(path) {
        Ok(content) => {
            if content.contains(pattern.as_str()) {
                println!("{path}: found {pattern:?}");
                Ok(())
            } else {
                println!("{path}: missing {pattern:?}");
                Err("pattern not found".into())
            }
        }
        Err(_) => {
            println!("{path}: not found");
            Err("file not readable".into())
        }
    }
}
