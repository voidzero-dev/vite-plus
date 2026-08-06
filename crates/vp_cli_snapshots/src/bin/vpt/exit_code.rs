use std::process::ExitStatus;

/// Local copy of `vp_shared::exit_code_from_status` (keep in sync).
/// Depending on `vp_shared` here would pull its full dependency tree into
/// this test-only crate and enable serde_json's `preserve_order` feature via
/// unification, changing `vpt json-edit` output order.
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
