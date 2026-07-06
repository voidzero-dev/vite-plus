use std::sync::{Arc, OnceLock};

/// Installs a Ctrl+C handler and returns the flag it sets, shared by the
/// ctrl-c task payloads (`exit-on-ctrlc`, `report-orphan-on-ctrlc`).
///
/// On Windows, an ancestor process (e.g. cargo, the test runner) may have
/// been created with `CREATE_NEW_PROCESS_GROUP`, which implicitly calls
/// `SetConsoleCtrlHandler(NULL, TRUE)` and sets `CONSOLE_IGNORE_CTRL_C` in
/// the PEB's ConsoleFlags. This flag is inherited by all descendants and
/// takes precedence over registered handlers — `CTRL_C_EVENT` is silently
/// dropped. Clear it so the handler can fire.
/// Ref: <https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags>
pub fn install_ctrlc_flag() -> Result<Arc<OnceLock<()>>, Box<dyn std::error::Error>> {
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

    let ctrlc_flag = Arc::new(OnceLock::new());
    ctrlc::set_handler({
        let ctrlc_flag = Arc::clone(&ctrlc_flag);
        move || {
            let _ = ctrlc_flag.set(());
        }
    })?;
    Ok(ctrlc_flag)
}
