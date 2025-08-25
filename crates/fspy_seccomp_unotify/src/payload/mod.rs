use std::os::fd::{RawFd};
mod filter;
use bincode::{Decode, Encode};
pub use filter::Filter;

#[derive(Debug, Encode, Decode, Clone)]
pub struct SeccompPayload {
    pub(crate) ipc_fd: RawFd,
    pub(crate) filter: Filter,
}
