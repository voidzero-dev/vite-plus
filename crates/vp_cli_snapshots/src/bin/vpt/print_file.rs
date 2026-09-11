/// print-file `<path>...`
///
/// Prints each file's bytes like cat, and like cat exits nonzero when any
/// operand is missing, so migrated `cat` assertions keep their shell exit
/// semantics under the runner's failure flow.
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write as _;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut failed = false;
    for file in args {
        match std::fs::read(file) {
            Ok(content) => out.write_all(&content)?,
            Err(_) => {
                eprintln!("{file}: not found");
                failed = true;
            }
        }
    }
    if failed {
        return Err("missing file".into());
    }
    Ok(())
}
