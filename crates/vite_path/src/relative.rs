//! Provides `RelativePath(Buf)`, a relative path type with additional guarantees to make it portable.
//!
//! ## Why not use crate `relative-path`
//! `relative-path::RelativePath` allows backslashes in its components, which is valid in unix systems but not portable to Windows.

use diff::Diff;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::{
    borrow::Borrow,
    ops::Deref,
    path::{Component, Path},
};
use vite_str::Str;

use bincode::{Decode, Encode, de::Decoder, error::DecodeError, impl_borrow_decode};

use ref_cast::{RefCastCustom, ref_cast_custom};

/// A relative path with additional guarantees to make it portable:
///
/// - It is valid utf-8
/// - It uses slashes `/` as separators, not backslashes `\`
/// - There's no backslash `\` in components (this is valid in unix systems but not portable to Windows)
#[derive(RefCastCustom, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RelativePath(str);
impl AsRef<RelativePath> for RelativePath {
    fn as_ref(&self) -> &RelativePath {
        &self
    }
}

impl AsRef<Path> for RelativePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl RelativePath {
    #[ref_cast_custom]
    unsafe fn assume_portable(path: &str) -> &Self;
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn as_path(&self) -> &Path {
        Path::new(self.as_str())
    }
    pub fn to_relative_path_buf(&self) -> RelativePathBuf {
        RelativePathBuf(self.0.into())
    }
    /// Creates an owned [`RelativePathBuf`] with `rel_path` adjoined to `self`.
    pub fn join<P: AsRef<RelativePath>>(&self, rel_path: P) -> RelativePathBuf {
        let mut absolute_path_buf = self.to_relative_path_buf();
        absolute_path_buf.push(rel_path);
        absolute_path_buf
    }

    /// Returns a path that, when joined onto `base`, yields `self`.
    ///
    /// If `base` is not a prefix of `self`, returns [`None`].
    pub fn strip_prefix<P: AsRef<RelativePath>>(&self, base: P) -> Option<&RelativePath> {
        let stripped_path = Path::new(self.as_str()).strip_prefix(base.as_ref().as_path()).ok()?;
        Some(unsafe { RelativePath::assume_portable(stripped_path.to_str().unwrap()) })
    }
}

/// A owned relative path buf with the same guarantees as `RelativePath`
#[derive(
    Debug,
    Encode,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Serialize,
    Deserialize,
    Default,
    Diff,
)]
#[diff(attr(#[derive(Debug)]))]
pub struct RelativePathBuf(Str);

impl AsRef<Path> for RelativePathBuf {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl Display for RelativePathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl PartialEq<RelativePath> for RelativePathBuf {
    fn eq(&self, other: &RelativePath) -> bool {
        self.as_relative_path().eq(other)
    }
}
impl PartialEq<&RelativePath> for RelativePathBuf {
    fn eq(&self, other: &&RelativePath) -> bool {
        self.as_relative_path().eq(other)
    }
}

impl RelativePathBuf {
    /// Extends `self` with `path`.
    ///
    /// Unlike [`std::path::PathBuf::push`], `self` and `path` are both always relative,
    /// so `self` can only be appended, not replaced
    pub fn push<P: AsRef<RelativePath>>(&mut self, rel_path: P) {
        let rel_path_str = rel_path.as_ref().as_str();
        if rel_path_str.is_empty() {
            return;
        }
        self.0.push('/');
        self.0.push_str(rel_path.as_ref().as_str());
    }

    /// Creates a new `RelativePathBuf` from a `Path`.
    ///
    /// This function normalizes the path by:
    /// - Removing `.` components
    /// - Replacing backslash `\` separators with slashes `/` (on Windows)
    ///
    /// # Errors
    /// Returns an error if the path is not relative or contains invalid data that makes it non-portable.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, FromPathError> {
        let path = path.as_ref();
        let mut path_str = Str::with_capacity(path.as_os_str().len());
        for component in path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => {
                    return Err(FromPathError::NonRelative);
                }
                Component::CurDir => {
                    // normalize dots
                    continue;
                }
                Component::ParentDir => {
                    path_str.push_str("..");
                }
                Component::Normal(os_str) => {
                    let Some(component) = os_str.to_str() else {
                        return Err(InvalidPathDataError::NonUtf8.into());
                    };
                    if component.contains('\\') {
                        return Err(InvalidPathDataError::BackslashInComponent.into());
                    }
                    path_str.push_str(component);
                }
            }
            path_str.push('/');
        }
        path_str.pop(); // remove last pushed '/'
        Ok(Self(path_str))
    }

    pub fn as_relative_path(&self) -> &RelativePath {
        unsafe { RelativePath::assume_portable(&self.0) }
    }
}

impl<'a, Context> Decode<Context> for RelativePathBuf {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let path_str = Str::decode(decoder)?;
        RelativePathBuf::new(path_str.as_str())
            .map_err(|err| DecodeError::OtherString(format!("{}: {}", err, path_str)))
    }
}
impl_borrow_decode!(RelativePathBuf);

impl AsRef<RelativePath> for RelativePathBuf {
    fn as_ref(&self) -> &RelativePath {
        self.as_relative_path()
    }
}

impl Deref for RelativePathBuf {
    type Target = RelativePath;

    fn deref(&self) -> &Self::Target {
        self.as_relative_path()
    }
}

impl Borrow<RelativePath> for RelativePathBuf {
    fn borrow(&self) -> &RelativePath {
        self.as_relative_path()
    }
}
impl ToOwned for RelativePath {
    type Owned = RelativePathBuf;
    fn to_owned(&self) -> Self::Owned {
        self.to_relative_path_buf()
    }
}

/// Error when converting a path containing invalid data to `RelativePathbuf`
#[derive(thiserror::Error, Debug)]
pub enum InvalidPathDataError {
    /// One of the components contains non-utf8 data.
    #[error("path is not portable because contains non-utf8 data")]
    NonUtf8,
    /// One of the components contains backslashes `\`.
    ///
    /// This is valid in unix systems but not portable to Windows
    #[error("path is not portable because it contains backslash ('\\') in its components")]
    BackslashInComponent,
}

/// Error when converting a `Path` to `RelativePathbuf`
#[derive(thiserror::Error, Debug)]
pub enum FromPathError {
    #[error("path is not relative")]
    NonRelative,
    #[error("{0}")]
    InvalidPathData(#[from] InvalidPathDataError),
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    use assert2::let_assert;

    use super::*;

    #[test]
    fn non_relative() {
        let_assert!(
            Err(FromPathError::NonRelative) =
                RelativePathBuf::new(if cfg!(windows) { "C:\\Users" } else { "/home" })
        );
    }
    #[test]
    fn non_utf8() {
        let non_utf8_path = Path::new(OsStr::from_bytes(&[0xC0]));
        let_assert!(
            Err(FromPathError::InvalidPathData(InvalidPathDataError::NonUtf8)) =
                RelativePathBuf::new(non_utf8_path),
        );
    }

    #[cfg(unix)]
    #[test]
    fn backslash_in_component() {
        let_assert!(
            Err(FromPathError::InvalidPathData(InvalidPathDataError::BackslashInComponent)) =
                RelativePathBuf::new("foo\\bar")
        );
    }

    #[cfg(windows)]
    #[test]
    fn replace_backslash_separators() {
        let rel_path = RelativePathBuf::new("foo\\bar").unwrap();
        assert_eq!(rel_path.as_str(), "foo/bar");
    }

    #[test]
    fn normalize_dots() {
        let rel_path = RelativePathBuf::new("./foo/./bar/.").unwrap();
        assert_eq!(rel_path.as_str(), "foo/bar")
    }

    #[test]
    fn normalize_trailing_slashes() {
        let rel_path = RelativePathBuf::new("foo/bar//").unwrap();
        assert_eq!(rel_path.as_str(), "foo/bar")
    }
    #[test]
    fn preserve_double_dots() {
        let rel_path = RelativePathBuf::new("../foo/../bar/..").unwrap();
        assert_eq!(rel_path.as_str(), "../foo/../bar/..")
    }

    #[test]
    fn push() {
        let mut rel_path = RelativePathBuf::new("foo/bar").unwrap();
        rel_path.push(RelativePathBuf::new(Path::new("baz")).unwrap());
        assert_eq!(rel_path.as_str(), "foo/bar/baz")
    }

    #[test]
    fn push_empty() {
        let mut rel_path = RelativePathBuf::new("foo/bar").unwrap();
        rel_path.push(RelativePathBuf::new("").unwrap());
        assert_eq!(rel_path.as_str(), "foo/bar")
    }

    #[test]
    fn join() {
        let rel_path = RelativePathBuf::new("foo/bar").unwrap();
        let joined_path = rel_path.as_relative_path().join(RelativePathBuf::new("baz").unwrap());
        assert_eq!(joined_path.as_str(), "foo/bar/baz")
    }

    #[test]
    fn strip_prefix() {
        let rel_path = RelativePathBuf::new("foo/bar/baz").unwrap();
        let prefix = RelativePathBuf::new("foo").unwrap();
        let stripped_path = rel_path.strip_prefix(prefix).unwrap();
        assert_eq!(stripped_path.as_str(), "bar/baz")
    }

    #[test]
    fn encode_decode() {
        let rel_path = RelativePathBuf::new("foo/bar").unwrap();
        let config = bincode::config::standard();
        let encoded = bincode::encode_to_vec(&rel_path, config).unwrap();
        let (decoded, _) =
            bincode::decode_from_slice::<RelativePathBuf, _>(&encoded, config).unwrap();
        assert_eq!(rel_path, decoded);
    }
}
