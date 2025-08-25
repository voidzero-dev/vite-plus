use crate::open_exec::open_executable;

use std::{path::Path};

use bstr::{BString, ByteSlice};

#[derive(Debug, Clone)]
pub struct Shebang {
    pub interpreter: BString,
    pub arguments: Vec<BString>,
}

fn is_whitespace(c: u8) -> bool {
    c == b' ' || c == b'\t'
}

#[derive(Clone, Copy, Debug)]
pub struct ParseShebangOptions {
    pub split_arguments: bool, // TODO: recursive
}

impl Default for ParseShebangOptions {
    fn default() -> Self {
        Self {
            split_arguments: cfg!(target_vendor = "apple"),
        }
    }
}

pub fn parse_shebang(
    mut peek_executable: impl FnMut(&Path, &mut [u8]) -> nix::Result<usize>,
    path: &Path,
    options: ParseShebangOptions,
) -> Result<Option<Shebang>, nix::Error> {
    // https://lwn.net/Articles/779997/
    // > The array used to hold the shebang line is defined to be 128 bytes in length
    // TODO: check linux/macOS' kernel source
    const PEEK_SIZE: usize = 128;

    let mut buf = [0u8; PEEK_SIZE];

    let total_read_size = peek_executable(path, &mut buf)?;

    let Some(buf) = buf[..total_read_size].strip_prefix(b"#!") else {
        return Ok(None);
    };

    let Some(buf) = buf.split(|ch| matches!(*ch, b'\n')).next() else {
        // https://github.com/torvalds/linux/blob/5723cc3450bccf7f98f227b9723b5c9f6b3af1c5/fs/binfmt_script.c#L59-L80
        return Err(nix::Error::ENOEXEC);
    };
    let buf = buf.trim_ascii();
    let Some(interpreter) = buf.split(|ch| is_whitespace(*ch)).next() else {
        return Ok(None);
    };
    let arguments_buf = buf[interpreter.len()..].trim_ascii_start().as_bstr();

    let arguments: Vec<BString> = if options.split_arguments {
        arguments_buf
            .split(|ch| is_whitespace(*ch))
            .flat_map(|arg| {
                let arg = arg.trim_ascii();
                if arg.is_empty() {
                    None
                } else {
                    Some(arg.as_bstr().to_owned())
                }
            })
            .collect()
    } else {
        if arguments_buf.is_empty() {
            vec![]
        } else {
            vec![arguments_buf.to_owned()]
        }
    };

    Ok(Some(Shebang {
        interpreter: interpreter.as_bstr().to_owned(),
        arguments,
    }))
}

// #[derive(Debug)]
// pub struct RecursiveParseOpts {
//     pub recursion_limit: usize,
//     pub split_arguments: bool,
// }

// impl Default for RecursiveParseOpts {
//     fn default() -> Self {
//         Self {
//             recursion_limit: 4, // BINPRM_MAX_RECURSION
//             split_arguments: false,
//         }
//     }
// }

// fn parse_shebang_recursive_impl<R: Read>(
//     buf: &mut [u8],
//     reader: R,
//     mut get_reader: impl FnMut(&OsStr) -> io::Result<R>,
//     mut on_shebang: impl FnMut(shebang<'_>) -> io::Result<()>,
// ) -> io::Result<()> {
//     let Some(mut shebang) = parse_shebang(buf, reader)? else {
//         return Ok(());
//     };
//     on_shebang(shebang)?;
//     loop {
//         let reader = get_reader(&shebang.interpreter)?;
//         let Some(cur_shebang) = parse_shebang(buf, reader)? else {
//             break Ok(());
//         };
//         on_shebang(cur_shebang)?;
//         shebang = cur_shebang;
//     }
// }

// pub fn parse_shebang_recursive<
//     const PEEK_CAP: usize,
//     R: Read,
//     O: FnMut(&OsStr) -> io::Result<R>,
//     C: FnMut(&OsStr) -> io::Result<()>,
// >(
//     opts: RecursiveParseOpts,
//     reader: R,
//     open: O,
//     mut on_arg_reverse: C,
// ) -> io::Result<()> {
//     let mut peek_buf = [0u8; PEEK_CAP];
//     let mut recursive_count = 0;
//     parse_shebang_recursive_impl(&mut peek_buf, reader, open, |shebang| {
//         if recursive_count > opts.recursion_limit {
//             return Err(io::Error::from_raw_os_error(libc::ELOOP));
//         }
//         if opts.split_arguments {
//             for arg in shebang.arguments.split().rev() {
//                 on_arg_reverse(arg)?;
//             }
//         } else {
//             on_arg_reverse(shebang.arguments.as_one())?;
//         }
//         on_arg_reverse(shebang.interpreter)?;
//         recursive_count += 1;
//         Ok(())
//     })?;
//     Ok(())
// }

// #[cfg(test)]
// mod tests {
//     use std::os::unix::ffi::OsStrExt;

//     use super::*;

//     #[test]
//     fn shebang_basic() {
//         let mut buf = [0u8; PEEK_SIZE];
//         let shebang = parse_shebang(&mut buf, "#!/bin/sh a b\n".as_bytes())
//             .unwrap()
//             .unwrap();
//         assert_eq!(shebang.interpreter.as_bytes(), b"/bin/sh");
//         assert_eq!(shebang.arguments.as_one().as_bytes(), b"a b");
//         assert_eq!(
//             shebang
//                 .arguments
//                 .split()
//                 .map(OsStrExt::as_bytes)
//                 .collect::<Vec<_>>(),
//             vec![b"a", b"b"]
//         );
//     }

//     #[test]
//     fn shebang_triming_spaces() {
//         let mut buf = [0u8; PEEK_SIZE];
//         let shebang = parse_shebang(&mut buf, "#! /bin/sh a \n".as_bytes())
//             .unwrap()
//             .unwrap();
//         assert_eq!(shebang.interpreter, "/bin/sh");
//         assert_eq!(shebang.arguments.as_one().as_bytes(), b"a");
//         assert_eq!(
//             shebang
//                 .arguments
//                 .split()
//                 .map(OsStrExt::as_bytes)
//                 .collect::<Vec<_>>(),
//             vec![b"a"]
//         );
//     }

//     #[test]
//     fn shebang_split_arguments() {
//         let mut buf = [0u8; PEEK_SIZE];
//         let shebang = parse_shebang(&mut buf, "#! /bin/sh a  b\tc \n".as_bytes())
//             .unwrap()
//             .unwrap();
//         assert_eq!(shebang.interpreter, "/bin/sh");
//         assert_eq!(
//             shebang
//                 .arguments
//                 .split()
//                 .map(OsStrExt::as_bytes)
//                 .collect::<Vec<_>>(),
//             &[b"a", b"b", b"c"]
//         );
//     }
//     #[test]
//     fn shebang_recursive_basic() {
//         let mut args = Vec::<String>::new();
//         parse_shebang_recursive::<PEEK_SIZE, _, _, _>(
//             RecursiveParseOpts {
//                 split_arguments: true,
//                 ..RecursiveParseOpts::default()
//             },
//             "#!/bin/B bparam".as_bytes(),
//             |path| {
//                 Ok(match path.as_bytes() {
//                     b"/bin/B" => "#! /bin/A aparam1 aparam2".as_bytes(),
//                     b"/bin/A" => "not a shebang script".as_bytes(),
//                     _ => unreachable!("Unexpected path: {}", path.display()),
//                 })
//             },
//             |arg| {
//                 args.push(str::from_utf8(arg.as_bytes()).unwrap().to_owned());
//                 Ok(())
//             },
//         )
//         .unwrap();
//         args.reverse();
//         assert_eq!(
//             args,
//             vec!["/bin/A", "aparam1", "aparam2", "/bin/B", "bparam"]
//         );
//     }
// }
