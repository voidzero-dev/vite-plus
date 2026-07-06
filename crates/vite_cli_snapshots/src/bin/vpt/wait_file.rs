/// wait-file `<path>` \[timeout-ms\]
///
/// Polls until `<path>` exists, then prints its contents. Fails after the
/// timeout (default 5000 ms). For asserting on files written asynchronously
/// by a previous step's process tree (e.g. a task outliving `vp run`).
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let Some(path) = args.first() else {
        return Err("Usage: vpt wait-file <path> [timeout-ms]".into());
    };
    let timeout_ms: u64 = match args.get(1) {
        Some(raw) => raw.parse()?,
        None => 5000,
    };

    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
    loop {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                print!("{content}");
                return Ok(());
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                if std::time::Instant::now() >= deadline {
                    return Err(format!("timed out waiting for file: {path}").into());
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(err) => return Err(err.into()),
        }
    }
}
