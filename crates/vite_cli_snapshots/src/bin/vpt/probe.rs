/// probe
///
/// Interactive payload for runner self-tests: proves milestone
/// synchronization, keystroke delivery, and screen capture end-to-end without
/// requiring milestone instrumentation in the product CLI. Prints a question,
/// marks `probe:ask`, reads a line, greets, then marks `probe:done`.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{BufRead as _, Write as _};

    println!("What is your name?");
    std::io::stdout().flush()?;
    pty_terminal_test_client::mark_milestone("probe:ask");

    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    let name = line.trim_end_matches(['\r', '\n']);

    println!("Hello, {name}!");
    std::io::stdout().flush()?;
    pty_terminal_test_client::mark_milestone("probe:done");
    Ok(())
}
