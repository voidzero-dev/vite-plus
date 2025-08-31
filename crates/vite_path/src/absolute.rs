use ref_cast::{RefCastCustom, ref_cast_custom};
use std::{
    fmt::Display,
    ops::Deref,
    path::{Path, PathBuf},
};

use crate::relative::{FromPathError, InvalidPathDataError, RelativePath, RelativePathBuf};

/// A path that is guaranteed to be absolute
#[derive(RefCastCustom, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct AbsolutePath(Path);
impl AsRef<AbsolutePath> for AbsolutePath {
    fn as_ref(&self) -> &AbsolutePath {
        &self
    }
}

impl AbsolutePath {
    /// Creates a [`AbsolutePath`] if the give path is absolute.
    pub fn new(path: &Path) -> Option<&Self> {
        if path.is_absolute() { Some(unsafe { Self::assume_absolute(path) }) } else { None }
    }

    #[ref_cast_custom]
    pub(crate) unsafe fn assume_absolute(abs_path: &Path) -> &Self;

    /// Gets the underlying [`Path`]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Converts `self` to an owned [`AbsolutePathBuf`].
    pub fn to_absolute_path_buf(&self) -> AbsolutePathBuf {
        unsafe { AbsolutePathBuf::assume_absolute(self.0.to_path_buf()) }
    }

    /// Returns a path that, when joined onto base, yields self.
    ///
    /// If `base` is not a prefix of `self`, returns [`None`].
    ///
    /// If the stripped path is not a valid [`RelativePath`]. Returns an error with the reason and the stripped path.
    pub fn strip_prefix<P: AsRef<AbsolutePath>>(
        &self,
        base: P,
    ) -> Result<Option<RelativePathBuf>, StripPrefixError<'_>> {
        let base = base.as_ref();
        let Ok(stripped_path) = self.0.strip_prefix(&base.0) else {
            return Ok(None);
        };
        match RelativePathBuf::try_from(stripped_path) {
            Ok(relative_path) => Ok(Some(relative_path)),
            Err(FromPathError::NonRelative) => {
                unreachable!("stripped path should always be relative")
            }
            Err(FromPathError::InvalidPathData(invalid_path_data_error)) => {
                Err(StripPrefixError { stripped_path, invalid_path_data_error })
            }
        }
    }

    /// Creates an owned [`AbsolutePathBuf`] with `rel_path` adjoined to `self`.
    pub fn join<P: AsRef<RelativePath>>(&self, rel_path: P) -> AbsolutePathBuf {
        let mut absolute_path_buf = self.to_absolute_path_buf();
        absolute_path_buf.push(rel_path);
        absolute_path_buf
    }
}

/// An Error returned from [`AbsolutePath::strip_prefix`] if the stripped path is not a valid `RelativePath`
#[derive(thiserror::Error, Debug)]
pub struct StripPrefixError<'a> {
    pub stripped_path: &'a Path,
    #[source]
    pub invalid_path_data_error: InvalidPathDataError,
}

impl Display for StripPrefixError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}: {}",
            self.stripped_path.display(),
            &self.invalid_path_data_error
        ))
    }
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

/// An owned path buf that is guaranteed to be absolute
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsolutePathBuf(PathBuf);

impl AbsolutePathBuf {
    pub fn new(path: PathBuf) -> Option<Self> {
        if path.is_absolute() { Some(unsafe { Self::assume_absolute(path) }) } else { None }
    }
    pub unsafe fn assume_absolute(abs_path: PathBuf) -> Self {
        Self(abs_path)
    }
    pub fn as_absolute_path(&self) -> &AbsolutePath {
        unsafe { AbsolutePath::assume_absolute(self.0.as_path()) }
    }

    /// Extends `self` with `path`.
    ///
    /// Unlike [`PathBuf::push`], `path` is always relative, so `self` can only be appended, not replaced.
    pub fn push<P: AsRef<RelativePath>>(&mut self, rel_path: P) {
        self.0.push(rel_path.as_ref().as_path());
    }
}

impl PartialEq<AbsolutePath> for AbsolutePathBuf {
    fn eq(&self, other: &AbsolutePath) -> bool {
        self.as_absolute_path().eq(other)
    }
}
impl PartialEq<&AbsolutePath> for AbsolutePathBuf {
    fn eq(&self, other: &&AbsolutePath) -> bool {
        self.as_absolute_path().eq(other)
    }
}

impl AsRef<Path> for AbsolutePathBuf {
    fn as_ref(&self) -> &Path {
        self.as_absolute_path().as_path()
    }
}
impl AsRef<AbsolutePath> for AbsolutePathBuf {
    fn as_ref(&self) -> &AbsolutePath {
        self.as_absolute_path()
    }
}

impl Deref for AbsolutePathBuf {
    type Target = AbsolutePath;

    fn deref(&self) -> &Self::Target {
        self.as_absolute_path()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    #[cfg(unix)]
    use assert2::let_assert;

    #[test]
    fn non_absolute() {
        assert!(AbsolutePath::new(Path::new("foo/bar")).is_none())
    }

    #[test]
    fn strip_prefix() {
        let abs_path = AbsolutePath::new(Path::new(if cfg!(windows) {
            "C:\\Users\\foo\\bar"
        } else {
            "/home/foo/bar"
        }))
        .unwrap();

        let prefix =
            AbsolutePath::new(Path::new(if cfg!(windows) { "C:\\Users" } else { "/home" }))
                .unwrap();

        let rel_path = abs_path.strip_prefix(prefix).unwrap().unwrap();
        assert_eq!(rel_path.as_str(), "foo/bar");

        assert_eq!(prefix.join(&rel_path), abs_path);
        let mut pushed_path = prefix.to_absolute_path_buf();
        pushed_path.push(rel_path);

        assert_eq!(pushed_path, abs_path);
    }

    #[test]
    fn strip_prefix_trailing_slash() {
        let abs_path = AbsolutePath::new(Path::new(if cfg!(windows) {
            "C:\\Users\\foo\\bar"
        } else {
            "/home/foo/bar"
        }))
        .unwrap();

        let prefix =
            AbsolutePath::new(Path::new(if cfg!(windows) { "C:\\Users\\" } else { "/home//" }))
                .unwrap();

        let rel_path = abs_path.strip_prefix(prefix).unwrap().unwrap();
        assert_eq!(rel_path.as_str(), "foo/bar");
    }

    #[test]
    fn strip_prefix_not_found() {
        let abs_path = AbsolutePath::new(Path::new(if cfg!(windows) {
            "C:\\Users\\foo\\bar"
        } else {
            "/home/foo/bar"
        }))
        .unwrap();

        let prefix = AbsolutePath::new(Path::new(if cfg!(windows) {
            "C:\\Users\\barz"
        } else {
            "/home/baz"
        }))
        .unwrap();

        let rel_path = abs_path.strip_prefix(prefix).unwrap();
        assert!(rel_path.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn strip_prefix_invalid_relative() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        let mut abs_path = b"/home/".to_vec();
        abs_path.push(0xC0);
        let abs_path = AbsolutePath::new(Path::new(OsStr::from_bytes(&abs_path))).unwrap();

        let prefix = AbsolutePath::new(Path::new("/home")).unwrap();
        let_assert!(Err(err) = abs_path.strip_prefix(prefix));

        assert_eq!(err.stripped_path.as_os_str().as_bytes(), &[0xC0]);
        let_assert!(InvalidPathDataError::NonUtf8 = err.invalid_path_data_error);
    }
}
