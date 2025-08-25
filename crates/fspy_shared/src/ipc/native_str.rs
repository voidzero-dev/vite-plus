#[cfg(windows)]
use std::ffi::OsString;
#[cfg(unix)]
use std::sync::Arc;
use std::{borrow::Cow, fmt::Debug};
use std::{ffi::OsStr, mem::MaybeUninit, path::Path};

use allocator_api2::alloc::Allocator;
#[cfg(unix)]
use bincode::Decode;
use bincode::{BorrowDecode, Encode};
use bstr::BStr;

/// Similar to OsStr, but requires no copy for encode/borrow_decode
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
    pub fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            #[cfg(windows)]
            is_wide: false,
            data: bytes,
        }
    }

    #[cfg(windows)]
    pub fn from_wide(wide: &'a [u16]) -> Self {
        use bytemuck::must_cast_slice;
        Self {
            is_wide: true,
            data: must_cast_slice(wide),
        }
    }

    #[cfg(unix)]
    pub fn as_os_str(&self) -> &'a OsStr {
        std::os::unix::ffi::OsStrExt::from_bytes(self.data)
    }

    #[cfg(unix)]
    pub fn as_bstr(&self) -> &'a BStr {
        use bstr::ByteSlice;

        self.data.as_bstr()
    }

    #[cfg(windows)]
    pub fn to_os_string(&self) -> OsString {
        use bytemuck::allocation::pod_collect_to_vec;
        use bytemuck::try_cast_slice;
        use std::os::windows::ffi::OsStringExt;
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

    pub fn to_cow_os_str(&self) -> Cow<'a, OsStr> {
        #[cfg(windows)]
        return Cow::Owned(self.to_os_string());
        #[cfg(unix)]
        return Cow::Borrowed(self.as_os_str());
    }
}

impl<'a> From<&'a BStr> for NativeStr<'a> {
    fn from(value: &'a BStr) -> Self {
        Self::from_bytes(&value)
    }
}

impl<'a> Debug for NativeStr<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <OsStr as Debug>::fmt(self.to_cow_os_str().as_ref(), f)
    }
}

#[cfg(unix)]
#[derive(Encode, Decode, Clone, Hash)]
pub struct NativeString {
    data: Arc<[u8]>,
}

#[cfg(unix)]
impl<'a> Debug for NativeString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <OsStr as Debug>::fmt(self.as_os_str(), f)
    }
}

#[cfg(unix)]
impl<'a> From<&'a OsStr> for NativeString {
    fn from(value: &'a OsStr) -> Self {
        use std::os::unix::ffi::OsStrExt;
        Self {
            data: value.as_bytes().into(),
        }
    }
}
#[cfg(unix)]
impl<'a> From<String> for NativeString {
    fn from(value: String) -> Self {
        Self {
            data: value.as_bytes().into(),
        }
    }
}
#[cfg(unix)]
impl<'a> From<&'a std::path::Path> for NativeString {
    fn from(value: &'a std::path::Path) -> Self {
        value.as_os_str().into()
    }
}

#[cfg(unix)]
impl std::ops::Deref for NativeString {
    type Target = OsStr;
    fn deref(&self) -> &Self::Target {
        self.as_os_str()
    }
}

#[cfg(unix)]
impl NativeString {
    pub fn as_os_str(&self) -> &OsStr {
        use std::os::unix::ffi::OsStrExt as _;
        OsStr::from_bytes(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn test_from_asni() {
        let asni_str = "hello";
        let native_str = NativeStr::from_bytes(asni_str.as_bytes());
        let os_string = native_str.to_os_string();
        assert_eq!(os_string.to_str().unwrap(), asni_str);
    }

    #[cfg(windows)]
    #[test]
    fn test_from_wide() {
        use std::os::windows::ffi::OsStrExt;

        use bincode::{borrow_decode_from_slice, config, decode_from_slice, encode_to_vec};

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
