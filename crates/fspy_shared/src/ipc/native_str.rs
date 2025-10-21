#[cfg(windows)]
use std::ffi::OsString;
use std::{
    borrow::Cow,
    ffi::OsStr,
    fmt::Debug,
    path::{Path, StripPrefixError},
    sync::Arc,
};

use allocator_api2::alloc::Allocator;
use bincode::{BorrowDecode, Decode, Encode};
#[cfg(unix)]
use bstr::BStr;

/// Similar to `OsStr`, but requires zero-copy to construct from either ansi or wide characters on Windows.
#[derive(Encode, BorrowDecode, Clone, Copy, PartialEq, Eq)]
pub struct NativeStr<'a> {
    #[cfg(windows)]
    is_wide: bool,
    data: &'a [u8],
}

#[cfg(unix)]
impl<'a> From<&'a Path> for NativeStr<'a> {
    fn from(value: &'a Path) -> Self {
        use std::os::unix::ffi::OsStrExt as _;
        Self::from_bytes(value.as_os_str().as_bytes())
    }
}

#[cfg(unix)]
impl<'a> From<&'a str> for NativeStr<'a> {
    #[cfg(unix)]
    fn from(value: &'a str) -> Self {
        Self::from_bytes(value.as_bytes())
    }
}

impl<'a> NativeStr<'a> {
    pub fn clone_in<'new_alloc, A>(&self, alloc: &'new_alloc A) -> NativeStr<'new_alloc>
    where
        &'new_alloc A: Allocator,
    {
        use allocator_api2::vec::Vec;
        let mut data = Vec::<u8, _>::with_capacity_in(self.data.len(), alloc);
        data.extend_from_slice(self.data);
        let data = data.leak::<'new_alloc>();
        NativeStr {
            data,
            #[cfg(windows)]
            is_wide: self.is_wide,
        }
    }

    #[cfg(unix)]
    #[must_use]
    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self { data: bytes }
    }

    #[cfg(windows)]
    #[must_use]
    pub const fn from_ansi(bytes: &'a [u8]) -> Self {
        Self { is_wide: false, data: bytes }
    }

    #[cfg(windows)]
    pub fn from_wide(wide: &'a [u16]) -> Self {
        use bytemuck::must_cast_slice;
        Self { is_wide: true, data: must_cast_slice(wide) }
    }

    #[cfg(unix)]
    #[must_use]
    pub fn as_os_str(&self) -> &'a OsStr {
        std::os::unix::ffi::OsStrExt::from_bytes(self.data)
    }

    #[cfg(unix)]
    #[must_use]
    pub fn as_bstr(&self) -> &'a BStr {
        use bstr::ByteSlice;

        self.data.as_bstr()
    }

    #[cfg(windows)]
    pub fn to_os_string(&self) -> OsString {
        use std::os::windows::ffi::OsStringExt;

        use bytemuck::{allocation::pod_collect_to_vec, try_cast_slice};
        use winsafe::{
            MultiByteToWideChar,
            co::{CP, MBC},
        };

        if self.is_wide {
            if let Ok(wide) = try_cast_slice::<u8, u16>(self.data) {
                OsString::from_wide(wide)
            } else {
                let wide = pod_collect_to_vec::<u8, u16>(self.data);
                OsString::from_wide(&wide)
            }
        } else {
            let wide = MultiByteToWideChar(CP::ACP, MBC::ERR_INVALID_CHARS, self.data).unwrap();
            OsString::from_wide(&wide)
        }
    }

    #[must_use]
    pub fn to_cow_os_str(&self) -> Cow<'a, OsStr> {
        #[cfg(windows)]
        return Cow::Owned(self.to_os_string());
        #[cfg(unix)]
        return Cow::Borrowed(self.as_os_str());
    }

    pub fn strip_path_prefix<P: AsRef<Path>, R, F: FnOnce(Result<&Path, StripPrefixError>) -> R>(
        &self,
        base: P,
        f: F,
    ) -> R {
        let me = self.to_cow_os_str();
        let me = strip_windows_path_prefix(&me);
        let base = strip_windows_path_prefix(base.as_ref().as_os_str());
        f(Path::new(me).strip_prefix(base))
    }
}

/// Strip the `\\?\`, `\\.\`, `\??\` prefix from a Windows path, if present.
/// Does nothing on non-Windows platforms.
///
/// \\?\ and \\.\ are used to enable long paths and access to device paths.
/// \??\ is used in Nt* calls.
/// The resulting path is not necessarily valid or points to the same location,
/// but it's good enough for sanitizing paths in `NativeStr::strip_path_prefix`.
fn strip_windows_path_prefix(p: &OsStr) -> &OsStr {
    #[cfg(windows)]
    {
        use os_str_bytes::OsStrBytesExt as _;
        for prefix in [r"\\?\", r"\\.\", r"\??\"] {
            if let Some(stripped) = p.strip_prefix(prefix) {
                return stripped;
            }
        }
        p
    }
    #[cfg(not(windows))]
    {
        p
    }
}

#[cfg(unix)]
impl<'a> From<&'a BStr> for NativeStr<'a> {
    fn from(value: &'a BStr) -> Self {
        Self::from_bytes(value)
    }
}

impl Debug for NativeStr<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <OsStr as Debug>::fmt(self.to_cow_os_str().as_ref(), f)
    }
}

/// Similar to `OsString`, but can be losslessly encoded/decoded using bincode.
/// `Encode`/`Decoded` implementations for `OsString` requires it to be valid UTF-8. This does not.
#[derive(Encode, Decode, Clone, Hash)]
pub struct NativeString {
    #[cfg(unix)]
    data: Arc<[u8]>,
    #[cfg(windows)]
    data: Arc<[u16]>,
}

impl NativeString {
    #[cfg(unix)]
    pub fn as_os_str(&self) -> &OsStr {
        use std::os::unix::ffi::OsStrExt as _;
        OsStr::from_bytes(&self.data)
    }

    #[cfg(windows)]
    pub fn to_os_string(&self) -> OsString {
        use std::os::windows::ffi::OsStringExt as _;
        OsString::from_wide(&self.data)
    }

    pub fn to_cow_os_str(&self) -> Cow<'_, OsStr> {
        #[cfg(unix)]
        return Cow::Borrowed(self.as_os_str());
        #[cfg(windows)]
        return Cow::Owned(self.to_os_string());
    }
}

impl<'a> Debug for NativeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <OsStr as Debug>::fmt(&self.to_cow_os_str(), f)
    }
}

impl<'a> From<&'a OsStr> for NativeString {
    #[cfg(unix)]
    fn from(value: &'a OsStr) -> Self {
        use std::os::unix::ffi::OsStrExt as _;
        Self { data: value.as_bytes().into() }
    }

    #[cfg(windows)]
    fn from(value: &'a OsStr) -> Self {
        use std::os::windows::ffi::OsStrExt as _;
        Self { data: value.encode_wide().collect() }
    }
}
// #[cfg(unix)]
// impl<'a> From<String> for NativeString {
//     fn from(value: String) -> Self {
//         Self { data: value.as_bytes().into() }
//     }
// }

impl<'a> From<&'a std::path::Path> for NativeString {
    fn from(value: &'a std::path::Path) -> Self {
        value.as_os_str().into()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::*;

    #[cfg(windows)]
    #[test]
    fn test_from_ansi() {
        let ansi_str = "hello";
        let native_str = NativeStr::from_ansi(ansi_str.as_bytes());
        let os_string = native_str.to_os_string();
        assert_eq!(os_string.to_str().unwrap(), ansi_str);
    }

    #[cfg(windows)]
    #[test]
    fn test_from_wide() {
        use std::os::windows::ffi::OsStrExt;

        use bincode::{borrow_decode_from_slice, config, encode_to_vec};

        let wide_str: &[u16] = &[528, 491];
        let native_str = NativeStr::from_wide(wide_str);

        let mut encoded = encode_to_vec(native_str, config::standard()).unwrap();

        let (decoded, _) =
            borrow_decode_from_slice::<'_, NativeStr<'_>, _>(&encoded, config::standard()).unwrap();
        let decoded_wide = decoded.to_os_string().encode_wide().collect::<Vec<u16>>();
        assert_eq!(decoded_wide, wide_str);

        let encoded_len = encoded.len();
        encoded.push(0);
        encoded.copy_within(..encoded_len, 1);

        let (decoded, _) =
            borrow_decode_from_slice::<'_, NativeStr<'_>, _>(&encoded[1..], config::standard())
                .unwrap();
        let decoded_wide = decoded.to_os_string().encode_wide().collect::<Vec<u16>>();
        assert_eq!(decoded_wide, wide_str);
    }
}
