use bincode::{Decode, Encode};

#[derive(Debug, Encode, Decode, Clone, Copy)]
pub(crate) struct CodableSockFilter {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
}

#[cfg(feature = "supervisor")]
impl From<seccompiler::sock_filter> for CodableSockFilter {
    fn from(c_filter: seccompiler::sock_filter) -> Self {
        let seccompiler::sock_filter { code, jt, jf, k } = c_filter;
        Self { code, jt, jf, k }
    }
}

#[cfg(feature = "target")]
impl From<CodableSockFilter> for libc::sock_filter {
    fn from(filter: CodableSockFilter) -> Self {
        let CodableSockFilter { code, jt, jf, k } = filter;
        libc::sock_filter { code, jt, jf, k }
    }
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct Filter(pub(crate) Vec<CodableSockFilter>);
