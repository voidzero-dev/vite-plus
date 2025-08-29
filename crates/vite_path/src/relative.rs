use std::{
    ops::Deref,
    path::{Component, Path},
};
use vite_str::Str;

use bincode::{Decode, Encode, de::Decoder, error::DecodeError};

use ref_cast::{RefCastCustom, ref_cast_custom};

/// A relative path with additional guarantees to make it portable:
///
/// - It is valid utf-8
/// - It uses slashes `/` as separators, not backslashes `\`
/// - There's no backslash `\` in components (this is valid in unix systems but not portable to Windows)
#[derive(RefCastCustom, PartialEq, Eq)]
#[repr(transparent)]
pub struct RelativePath(str);
impl AsRef<RelativePath> for RelativePath {
    fn as_ref(&self) -> &RelativePath {
        &self
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
#[derive(Debug, Encode, PartialEq, Eq)]
pub struct RelativePathBuf(Str);

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
        self.0.push('/');
        self.0.push_str(rel_path.as_ref().as_str());
    }

    pub fn as_relative_path(&self) -> &RelativePath {
        unsafe { RelativePath::assume_portable(&self.0) }
    }
}

impl<'a, Context> Decode<Context> for RelativePathBuf {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        let mut path_str = Str::decode(decoder)?;
        // checks relativity and slashes
        if Path::new(&path_str).is_absolute() {
            return Err(DecodeError::OtherString(format!(
                "tried to decode a `RelativePath` from an absolute path: {path_str}"
            )));
        }
        if path_str.contains('\\') {
            return Err(DecodeError::OtherString(format!(
                "tried to decode a `RelativePath` from a string with backslashes: {path_str}"
            )));
        }
        while path_str.ends_with('/') {
            path_str.pop();
        }
        Ok(Self(path_str))
    }
}

impl TryFrom<&Path> for RelativePathBuf {
    type Error = FromPathError;
    fn try_from(path: &Path) -> Result<Self, Self::Error> {
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
}

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
    use std::ffi::OsStr;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(windows)]
    use std::os::windows::ffi::OsStrExt;

    use assert2::let_assert;

    use super::*;

    #[test]
    fn non_relative() {
        let abs_path = Path::new(if cfg!(windows) { "C:\\Users" } else { "/home" });
        let_assert!(Err(FromPathError::NonRelative) = RelativePathBuf::try_from(abs_path));
    }

    #[test]
    #[cfg(unix)]
    fn non_utf8_unix() {
        // On Unix, paths can contain arbitrary bytes
        let non_utf8_path = Path::new(OsStr::from_bytes(&[0xC0]));
        let_assert!(
            Err(FromPathError::InvalidPathData(InvalidPathDataError::NonUtf8)) =
                RelativePathBuf::try_from(non_utf8_path),
        );
    }

    #[test]
    #[cfg(windows)]
    fn non_utf8_windows() {
        // On Windows, create invalid UTF-16 sequence
        let invalid_utf16 = [0xD800]; // Unpaired surrogate
        let non_utf8_path = Path::new(OsStr::from_wide(&invalid_utf16));
        let_assert!(
            Err(FromPathError::InvalidPathData(InvalidPathDataError::NonUtf8)) =
                RelativePathBuf::try_from(non_utf8_path),
        );
    }

    #[cfg(unix)]
    #[test]
    fn backslash_in_component() {
        let path = Path::new("foo\\bar");
        let_assert!(
            Err(FromPathError::InvalidPathData(InvalidPathDataError::BackslashInComponent)) =
                RelativePathBuf::try_from(path)
        );
    }

    #[cfg(windows)]
    #[test]
    fn replace_backslash_separators() {
        let path = Path::new("foo\\bar");
        let rel_path = RelativePathBuf::try_from(path).unwrap();
        assert_eq!(rel_path.as_str(), "foo/bar");
    }

    #[test]
    fn normalize_dots() {
        let path = Path::new("./foo/./bar/.");
        let rel_path = RelativePathBuf::try_from(path).unwrap();
        assert_eq!(rel_path.as_str(), "foo/bar")
    }

    #[test]
    fn normalize_trailing_slashes() {
        let path = Path::new("foo/bar//");
        let rel_path = RelativePathBuf::try_from(path).unwrap();
        assert_eq!(rel_path.as_str(), "foo/bar")
    }
    #[test]
    fn preserve_double_dots() {
        let path = Path::new("../foo/../bar/..");
        let rel_path = RelativePathBuf::try_from(path).unwrap();
        assert_eq!(rel_path.as_str(), "../foo/../bar/..")
    }

    #[test]
    fn push() {
        let mut rel_path = RelativePathBuf::try_from(Path::new("foo/bar")).unwrap();
        rel_path.push(RelativePathBuf::try_from(Path::new("baz")).unwrap());
        assert_eq!(rel_path.as_str(), "foo/bar/baz")
    }

    #[test]
    fn join() {
        let rel_path = RelativePathBuf::try_from(Path::new("foo/bar")).unwrap();
        let joined_path =
            rel_path.as_relative_path().join(RelativePathBuf::try_from(Path::new("baz")).unwrap());
        assert_eq!(joined_path.as_str(), "foo/bar/baz")
    }

    #[test]
    fn strip_prefix() {
        let rel_path = RelativePathBuf::try_from(Path::new("foo/bar/baz")).unwrap();
        let prefix = RelativePathBuf::try_from(Path::new("foo")).unwrap();
        let stripped_path = rel_path.strip_prefix(prefix).unwrap();
        assert_eq!(stripped_path.as_str(), "bar/baz")
    }

    #[test]
    fn encode_decode() {
        let rel_path = RelativePathBuf::try_from(Path::new("foo/bar")).unwrap();
        let config = bincode::config::standard();
        let encoded = bincode::encode_to_vec(&rel_path, config).unwrap();
        let (decoded, _) =
            bincode::decode_from_slice::<RelativePathBuf, _>(&encoded, config).unwrap();
        assert_eq!(rel_path, decoded);
    }
}
