use std::process::ExitStatus;

/// Convert a process status to the shell-compatible exit code used by CLI callers.
pub fn exit_code_from_status(status: ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::exit_code_from_status;

    /// Regression test for https://github.com/voidzero-dev/vite-plus/issues/2041.
    #[cfg(unix)]
    #[test]
    fn preserves_signal_exit_code() {
        let status =
            std::process::Command::new("/bin/sh").arg("-c").arg("kill -ILL $$").status().unwrap();
        assert_eq!(exit_code_from_status(status), 132);
    }
}
