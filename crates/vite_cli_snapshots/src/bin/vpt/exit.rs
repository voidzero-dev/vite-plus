pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let code: i32 = args.first().map(|s| s.parse()).transpose()?.unwrap_or(0);
    std::process::exit(code);
}
