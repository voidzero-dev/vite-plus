pub fn run() {
    use std::io::IsTerminal as _;
    let stdin_tty = if std::io::stdin().is_terminal() { "tty" } else { "not-tty" };
    let stdout_tty = if std::io::stdout().is_terminal() { "tty" } else { "not-tty" };
    let stderr_tty = if std::io::stderr().is_terminal() { "tty" } else { "not-tty" };
    println!("stdin:{stdin_tty}");
    println!("stdout:{stdout_tty}");
    println!("stderr:{stderr_tty}");
}
