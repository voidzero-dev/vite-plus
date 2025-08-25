use std::mem::{MaybeUninit, transmute};

use bstr::{BStr, ByteSlice};
use stackalloc::alloca;

fn concat<R>(s: &[&BStr], callback: impl FnOnce(&BStr) -> R) -> R {
    let size = s.iter().map(|s| s.len()).sum();
    alloca(size, |buf| {
        debug_assert_eq!(buf.len(), size);
        let mut pos = 0usize;
        for s in s {
            let next_pos = pos + s.len();
            buf[pos..next_pos]
                .copy_from_slice(unsafe { transmute::<&[u8], &[MaybeUninit<u8>]>(s.as_ref()) });
            pos = next_pos;
        }
        debug_assert_eq!(pos, buf.len());
        callback(unsafe { transmute::<&[MaybeUninit<u8>], &[u8]>(buf) }.as_bstr())
    })
}

const NAME_MAX: usize = 255;

/// Search the executable in PATH.
///
/// Referenced musl Implementation: https://github.com/kraj/musl/blob/1b06420abdf46f7d06ab4067e7c51b8b63731852/src/process/execvp.c#L5
///
/// Difference from musl:
/// - Instead of actually calling execve, use `access_executable` to check if the file is executable, and call `callback` with the found executable.
/// - The path limit (PATH_MAX) is not checked.
/// - PATH is passed as parameter instead of using the real environment variable.
pub fn which<R>(
    file: &BStr,
    path: &BStr,
    mut access_executable: impl FnMut(&BStr) -> nix::Result<()>,
    callback: impl FnOnce(&BStr) -> nix::Result<R>,
) -> nix::Result<R> {
    use nix::Error;
    // 1. If file is empty, return ENOENT
    if file.is_empty() {
        return Err(Error::ENOENT);
    }
    // 2. If file contains '/', call callback directly
    if file.contains(&b'/') {
        return callback(file);
    }
    // 3. If file is too long, return ENAMETOOLONG
    if file.len() > NAME_MAX {
        return Err(Error::ENAMETOOLONG);
    }
    // 4. Search PATH
    let mut seen_eacces = false;
    let mut last_err = Error::ENOENT;
    let mut callback = Some(callback);
    for p in path.split(|ch| *ch == b':') {
        let p = p.as_bstr();
        let result_to_return = concat(
            // join with '/' if path is not empty
            &[p, (if p.is_empty() { "" } else { "/" }).into(), file],
            |path| match access_executable(path) {
                Ok(()) => Some((callback.take().unwrap())(path)),
                Err(err @ (Error::EACCES | Error::ENOENT | Error::ENOTDIR)) => {
                    if err == Error::EACCES {
                        seen_eacces = true;
                    }
                    last_err = err;
                    None
                }
                Err(other_err) => Some(Err(other_err)),
            },
        );
        if let Some(result) = result_to_return {
            return result;
        }
    }
    Err(if seen_eacces { Error::EACCES } else { last_err })
}

#[cfg(test)]
mod tests {
    use super::*;

    use bstr::{B, BStr};
    use std::cell::RefCell;

    #[test]
    fn test_concat() {
        let s = concat(&["a".into(), "bc".into(), "".into(), "e".into()], |s| {
            s.to_owned()
        });
        assert_eq!(s, "abce");
    }

    fn mock_access<'a>(
        allowed: &'a [&'a str],
        fail_eacces: &'a [&'a str],
        fail_enotdir: &'a [&'a str],
    ) -> impl FnMut(&BStr) -> nix::Result<()> + 'a {
        move |path| {
            let s = std::str::from_utf8(path).unwrap();
            if allowed.contains(&s) {
                Ok(())
            } else if fail_eacces.contains(&s) {
                Err(nix::Error::EACCES)
            } else if fail_enotdir.contains(&s) {
                Err(nix::Error::ENOTDIR)
            } else {
                Err(nix::Error::ENOENT)
            }
        }
    }

    #[test]
    fn test_which_found() {
        let called = RefCell::new(None);
        let file: &BStr = B("foo").as_bstr();
        let path: &BStr = B("/bin:/usr/bin").as_bstr();
        let access = mock_access(&["/bin/foo"], &[], &[]);
        let res = which(&file, &path, access, |found| {
            *called.borrow_mut() = Some(found.to_owned());
            Ok(())
        });
        assert!(res.is_ok());
        assert_eq!(&*called.borrow().as_ref().unwrap(), b"/bin/foo".as_bstr());
    }

    #[test]
    fn test_which_not_found() {
        let file: &BStr = B("foo").as_bstr();
        let path: &BStr = B("/bin:/usr/bin").as_bstr();
        let access = mock_access(&[], &[], &[]);
        let res = which(&file, &path, access, |_| Ok(()));
        assert_eq!(res.unwrap_err(), nix::Error::ENOENT);
    }

    #[test]
    fn test_which_eacces() {
        let file: &BStr = B("foo").as_bstr();
        let path: &BStr = B("/bin:/usr/bin").as_bstr();
        let access = mock_access(&[], &["/bin/foo", "/usr/bin/foo"], &[]);
        let res = which(&file, &path, access, |_| Ok(()));
        assert_eq!(res.unwrap_err(), nix::Error::EACCES);
    }

    #[test]
    fn test_which_enotdir() {
        let file: &BStr = B("foo").as_bstr();
        let path: &BStr = B("/usr/bin:/bin").as_bstr();
        let access = mock_access(&[], &[], &["/bin/foo"]);
        let res = which(&file, &path, access, |_| Ok(()));
        assert_eq!(res.unwrap_err(), nix::Error::ENOTDIR);
    }

    #[test]
    fn test_which_slash_in_file() {
        let called = RefCell::new(None);
        let file: &BStr = B("/usr/bin/foo").as_bstr();
        let path: &BStr = B("").as_bstr();
        let access = mock_access(&["/usr/bin/foo"], &[], &[]);
        let res = which(&file, &path, access, |found| {
            *called.borrow_mut() = Some(found.to_owned());
            Ok(())
        });
        assert!(res.is_ok());
        assert_eq!(
            &*called.borrow().as_ref().unwrap(),
            b"/usr/bin/foo".as_bstr()
        );
    }

    #[test]
    fn test_which_empty_file() {
        let file: &BStr = B("").as_bstr();
        let path: &BStr = B("/bin:/usr/bin").as_bstr();
        let access = mock_access(&[], &[], &[]);
        let res = which(&file, &path, access, |_| Ok(()));
        assert_eq!(res.unwrap_err(), nix::Error::ENOENT);
    }

    #[test]
    fn test_which_file_too_long() {
        let long_name = vec![b'a'; NAME_MAX + 1];
        let file: &BStr = B(&long_name).as_bstr();
        let path: &BStr = B("/bin:/usr/bin").as_bstr();
        let access = mock_access(&[], &[], &[]);
        let res = which(&file, &path, access, |_| Ok(()));
        assert_eq!(res.unwrap_err(), nix::Error::ENAMETOOLONG);
    }

    #[test]
    fn test_which_empty_path_entry() {
        let called = RefCell::new(None);
        let file: &BStr = B("foo").as_bstr();
        let path: &BStr = B(":/bin").as_bstr();
        let access = mock_access(&["foo"], &[], &[]);
        let res = which(&file, &path, access, |found| {
            *called.borrow_mut() = Some(found.to_owned());
            Ok(())
        });
        assert!(res.is_ok());
        assert_eq!(&*called.borrow().as_ref().unwrap(), b"foo".as_bstr());
    }
}
