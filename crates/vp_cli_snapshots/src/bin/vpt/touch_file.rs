pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        return Err("Usage: vpt touch-file <filename>...".into());
    }
    // Unlike vtt's variant, this creates missing files: vpt documents
    // touch-file as the cross-platform `touch` replacement for fixtures.
    // Every operand is created, matching `touch a b`.
    for file in args {
        let _file = std::fs::OpenOptions::new().append(true).create(true).open(file)?;
    }
    Ok(())
}
