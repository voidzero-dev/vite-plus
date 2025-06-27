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
use compact_str::CompactString;
use diff::Diff;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default, Hash, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Str(CompactString);

impl Diff for Str {
    type Repr = Option<Str>;

    fn diff(&self, other: &Self) -> Self::Repr {
        if self != other { Some(other.clone()) } else { None }
    }

    fn apply(&mut self, diff: &Self::Repr) {
        if let Some(diff) = diff {
            *self = diff.clone()
        }
    }

    fn identity() -> Self {
        Str::default()
    }
}

impl Str {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
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
        self.0.deref()
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
        Ok(Str(compact_str))
    }
}
impl_borrow_decode!(Str);

impl<'a> From<&'a str> for Str {
    fn from(value: &'a str) -> Self {
        Str(value.into())
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
