pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() {
        return Err("Usage: vpt print-env <VAR_NAME>".into());
    }
    let value = std::env::var(&args[0]).unwrap_or_else(|_| "(undefined)".to_string());
    println!("{value}");
    Ok(())
}
