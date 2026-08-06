/// barrier `<dir>` `<prefix>` `<count>` \[--exit=`<code>`\] \[--hang\]
///
/// Cross-platform concurrency barrier for testing.
/// Creates `<dir>/<prefix>_<pid>`, then polls until `<count>` files matching
/// `<prefix>_*` exist in `<dir>`.
///
/// Options:
/// - `--exit=<code>`: Exit with the given code after the barrier is met.
/// - `--hang`: Keep process alive after the barrier (for kill tests).
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut positional: Vec<&str> = Vec::new();
    let mut exit_code: i32 = 0;
    let mut hang = false;

    for arg in args {
        if let Some(code) = arg.strip_prefix("--exit=") {
            exit_code = code.parse()?;
        } else if arg == "--hang" {
            hang = true;
        } else {
            positional.push(arg.as_str());
        }
    }

    if positional.len() < 3 {
        return Err("Usage: vpt barrier <dir> <prefix> <count> [--exit=<code>] [--hang]".into());
    }

    let dir = std::path::Path::new(positional[0]);
    let prefix = positional[1];
    let count: usize = positional[2].parse()?;

    std::fs::create_dir_all(dir)?;

    // Create this participant's marker file.
    let pid = std::process::id();
    let marker = dir.join(std::format!("{prefix}_{pid}"));
    std::fs::write(&marker, "")?;

    // Wait until <count> matching files exist. Polling keeps this dependency-free;
    // barrier latency is bounded by the poll interval, which is fine for tests.
    let prefix_match = std::format!("{prefix}_");
    let count_matches = |d: &std::path::Path| -> Result<bool, Box<dyn std::error::Error>> {
        Ok(std::fs::read_dir(d)?
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with(prefix_match.as_str()))
            .count()
            >= count)
    };
    while !count_matches(dir)? {
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    if hang {
        loop {
            std::thread::park();
        }
    }

    std::process::exit(exit_code);
}
