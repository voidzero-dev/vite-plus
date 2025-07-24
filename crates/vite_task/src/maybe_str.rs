use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use bincode::{Decode, Encode};
use bstr::BStr;
use serde::Serialize;

/// Similar to `bstr::BString`, but also implements `bincode::{Encode`, Decode},
/// and serializes losslessly to utf8 for outputting debug json

#[derive(Encode, Decode)]
pub struct MaybeString(Vec<u8>);

impl From<Vec<u8>> for MaybeString {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl Deref for MaybeString {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for MaybeString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Debug for MaybeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(BStr::new(&self.0), f)
    }
}

impl Serialize for MaybeString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&bstr::ByteSlice::escape_bytes(self.0.as_slice()))
    }
}
