pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{Read as _, Write as _};
    let mut stdin = std::io::stdin().lock();
    let mut stdout = std::io::stdout().lock();
    let mut buf = [0u8; 8192];
    loop {
        match stdin.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => stdout.write_all(&buf[..n])?,
        }
    }
    Ok(())
}
