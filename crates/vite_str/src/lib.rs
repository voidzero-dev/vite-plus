use std::{
    borrow::Borrow,
    ffi::OsStr,
    fmt::{Debug, Display},
    ops::Deref,
    path::Path,
    str::from_utf8,
};

use bincode::{
    Decode, Encode,
    de::{Decoder, read::Reader},
    enc::Encoder,
    error::{DecodeError, EncodeError},
    impl_borrow_decode,
};
use compact_str::{CompactString};
use diff::Diff;
use serde::{Deserialize, Serialize};

#[doc(hidden)] // for `format` macro only
pub use compact_str::format_compact;

#[macro_export]
macro_rules! format {
    ($($arg:tt)*) => {
        $crate::Str::from($crate::format_compact!($($arg)*))
    };
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default, Hash, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Str(CompactString);

impl Diff for Str {
    type Repr = Option<Self>;

    fn diff(&self, other: &Self) -> Self::Repr {
        if self == other { None } else { Some(other.clone()) }
    }

    fn apply(&mut self, diff: &Self::Repr) {
        if let Some(diff) = diff {
            *self = diff.clone();
        }
    }

    fn identity() -> Self {
        Self::default()
    }
}

impl Str {
    pub fn with_capacity(capacity: usize) -> Self {
        Self(CompactString::with_capacity(capacity))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    pub fn push(&mut self, ch: char) {
        self.0.push(ch);
    }
    pub fn pop(&mut self) -> Option<char> {
        self.0.pop()
    }
    pub fn push_str(&mut self, s: &str) {
        self.0.push_str(s);
    }
}

impl AsRef<str> for Str {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
impl AsRef<Path> for Str {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}
impl AsRef<OsStr> for Str {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}
impl Borrow<str> for Str {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}
impl Deref for Str {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Str {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
impl Debug for Str {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Encode for Str {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        self.0.encode(encoder)
    }
}

// https://github.com/bincode-org/bincode/blob/48ac8d4e8057387696a7ed3af2dda198ead23246/src/de/mod.rs#L331
fn decode_slice_len<D: Decoder>(decoder: &mut D) -> Result<usize, DecodeError> {
    let v = u64::decode(decoder)?;
    v.try_into().map_err(|_| DecodeError::OutsideUsizeRange(v))
}
impl<Context> Decode<Context> for Str {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        let len = decode_slice_len(decoder)?;
        decoder.claim_container_read::<u8>(len)?;

        let mut compact_str = CompactString::with_capacity(len);
        unsafe {
            let buf = &mut compact_str.as_mut_bytes()[..len];
            decoder.reader().read(buf)?;
            from_utf8(buf).map_err(|utf8_error| DecodeError::Utf8 { inner: utf8_error })?;
            compact_str.set_len(len);
        }
        Ok(Self(compact_str))
    }
}
impl_borrow_decode!(Str);

impl From<&str> for Str {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for Str {
    fn from(value: String) -> Self {
        Self(value.into())
    }
}

impl From<CompactString> for Str {
    fn from(value: CompactString) -> Self {
        Self(value)
    }
}

impl PartialEq<&str> for Str {
    fn eq(&self, other: &&str) -> bool {
        self.0 == other
    }
}
impl PartialEq<str> for Str {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::{config::standard, decode_from_slice, encode_to_vec};

    #[test]
    fn test_str_encode_decode() {
        let config = standard();
        let original = Str::from("Hello, World!");
        let encoded = encode_to_vec(&original, config).unwrap();

        let decoded: Str = decode_from_slice(&encoded, config).unwrap().0;
        assert_eq!(original, decoded);
    }
}
