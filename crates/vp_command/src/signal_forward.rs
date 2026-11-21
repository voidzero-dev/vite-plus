//! Forward termination signals from a wrapper process to the child it waits for.

use std::{
    mem::MaybeUninit,
    sync::atomic::{AtomicI32, Ordering},
};

use nix::{
    libc,
    sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction},
};

/// The child that receives forwarded signals, or 0 when no guard is alive.
static FORWARD_PID: AtomicI32 = AtomicI32::new(0);

/// While alive, sends SIGTERM and SIGHUP that reach this process on to the child.
///
/// A process manager or `kill <pid>` signals only this process. Without this
/// guard, this process exits and the child keeps running. SIGINT is not
/// forwarded, because the terminal sends Ctrl+C to the whole process group.
pub(crate) struct SignalForwardGuard {
    previous: Vec<(Signal, SigAction)>,
}

impl SignalForwardGuard {
    /// Returns `None` when another guard already forwards to a child.
    pub(crate) fn install(pid: u32) -> Option<Self> {
        let pid = i32::try_from(pid).ok()?;
        FORWARD_PID.compare_exchange(0, pid, Ordering::SeqCst, Ordering::SeqCst).ok()?;
        let action =
            SigAction::new(SigHandler::Handler(forward), SaFlags::SA_RESTART, SigSet::empty());
        let previous = [Signal::SIGTERM, Signal::SIGHUP]
            .into_iter()
            // Keep a signal that the caller ignores, such as SIGHUP under `nohup`.
            .filter(|&signal| !is_ignored(signal))
            .filter_map(|signal| {
                // SAFETY: `forward` only calls async-signal-safe functions.
                let previous = unsafe { sigaction(signal, &action) }.ok()?;
                Some((signal, previous))
            })
            .collect();
        Some(Self { previous })
    }
}

impl Drop for SignalForwardGuard {
    fn drop(&mut self) {
        // The child has exited. Stop forwarding before its PID can be reused.
        FORWARD_PID.store(0, Ordering::SeqCst);
        for (signal, previous) in self.previous.drain(..) {
            // SAFETY: reinstalls the disposition captured by `install`.
            let _ = unsafe { sigaction(signal, &previous) };
        }
    }
}

/// Reads the disposition without replacing it.
fn is_ignored(signal: Signal) -> bool {
    let mut current = MaybeUninit::<libc::sigaction>::uninit();
    // SAFETY: a null action only writes the current disposition to `current`.
    unsafe {
        libc::sigaction(signal as libc::c_int, std::ptr::null(), current.as_mut_ptr()) == 0
            && current.assume_init().sa_sigaction == libc::SIG_IGN
    }
}

extern "C" fn forward(signal: libc::c_int) {
    let pid = FORWARD_PID.load(Ordering::SeqCst);
    if pid > 0 {
        // SAFETY: `kill` is async-signal-safe.
        unsafe { libc::kill(pid, signal) };
    }
}
