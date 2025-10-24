mod filter;
use bincode::{Decode, Encode};
pub use filter::Filter;

#[derive(Debug, Encode, Decode, Clone)]
pub struct SeccompPayload {
    pub(crate) ipc_path: Vec<u8>,
    pub(crate) filter: Filter,
}
