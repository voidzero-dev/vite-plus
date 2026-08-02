use std::process::ExitStatus;

/// Convert a process status to the shell-compatible exit code used by CLI callers.
#[expect(clippy::disallowed_methods, reason = "sole sanctioned `ExitStatus::code` caller")]
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

    #[cfg(unix)]
    #[test]
    fn preserves_normal_exit_code() {
        let status =
            std::process::Command::new("/bin/sh").arg("-c").arg("exit 42").status().unwrap();
        assert_eq!(exit_code_from_status(status), 42);
    }

    #[cfg(windows)]
    #[test]
    fn preserves_normal_exit_code() {
        let status = std::process::Command::new("cmd").args(["/C", "exit 42"]).status().unwrap();
        assert_eq!(exit_code_from_status(status), 42);
    }

    /// Regression test for https://github.com/voidzero-dev/vite-plus/issues/2041.
    #[cfg(unix)]
    #[test]
    fn preserves_signal_exit_code() {
        let status =
            std::process::Command::new("/bin/sh").arg("-c").arg("kill -ILL $$").status().unwrap();
        assert_eq!(exit_code_from_status(status), 132);
    }
}
