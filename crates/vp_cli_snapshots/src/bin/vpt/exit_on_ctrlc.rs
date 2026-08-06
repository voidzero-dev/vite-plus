use std::sync::Arc;

/// exit-on-ctrlc
///
/// Sets up a Ctrl+C handler, emits a "ready" milestone, then waits.
/// When Ctrl+C is received, prints "ctrl-c received" and exits.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // On Windows, an ancestor process (e.g. cargo, the test runner) may have
    // been created with CREATE_NEW_PROCESS_GROUP, which implicitly calls
    // SetConsoleCtrlHandler(NULL, TRUE) and sets CONSOLE_IGNORE_CTRL_C in the
    // PEB's ConsoleFlags. This flag is inherited by all descendants and takes
    // precedence over registered handlers — CTRL_C_EVENT is silently dropped.
    // Clear it so our handler can fire.
    // Ref: https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags
    #[cfg(windows)]
    {
        // SAFETY: Passing (None, FALSE) clears the per-process CTRL_C ignore flag.
        unsafe extern "system" {
            fn SetConsoleCtrlHandler(
                handler: Option<unsafe extern "system" fn(u32) -> i32>,
                add: i32,
            ) -> i32;
        }
        // SAFETY: Clearing the inherited ignore flag.
        unsafe {
            SetConsoleCtrlHandler(None, 0);
        }
    }

    let ctrlc_once_lock = Arc::new(std::sync::OnceLock::<()>::new());

    ctrlc::set_handler({
        let ctrlc_once_lock = Arc::clone(&ctrlc_once_lock);
        move || {
            let _ = ctrlc_once_lock.set(());
        }
    })?;

    pty_terminal_test_client::mark_milestone("ready");

    ctrlc_once_lock.wait();
    println!("ctrl-c received");
    Ok(())
}
