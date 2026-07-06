/// exit-on-ctrlc
///
/// Sets up a Ctrl+C handler, emits a "ready" milestone, then waits.
/// When Ctrl+C is received, prints "ctrl-c received" and exits.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let ctrlc_flag = crate::ctrlc_util::install_ctrlc_flag()?;

    pty_terminal_test_client::mark_milestone("ready");

    ctrlc_flag.wait();
    println!("ctrl-c received");
    Ok(())
}
