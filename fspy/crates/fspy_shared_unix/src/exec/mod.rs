mod fs;
mod shebang;
mod which;

use bstr::{BStr, BString, ByteSlice};
use fspy_shared::ipc::{AccessMode, PathAccess};
use nix::unistd::{AccessFlags, access};

use std::{
    ffi::{CStr, OsStr, OsString},
    io,
    iter::once,
    mem::replace,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf, absolute},
};

use shebang::{ParseShebangOptions, parse_shebang};

use crate::open_exec::open_executable;

#[derive(Debug, Clone)]
pub struct SearchPath<'a> {
    /// Custom search path to use (like execvP), overrides PATH if Some
    pub custom_path: Option<&'a BStr>,
}

/// Configuration for exec resolution behavior
#[derive(Debug, Clone)]
pub struct ExecResolveConfig<'a> {
    /// If Some and the program doesn't contains `/`,
    /// search the program in PATH (like execvp, execvpe, execlp) instead of finding it in current directory
    pub search_path: Option<SearchPath<'a>>,
    /// Options for parsing shebangs (all exec variants handle shebangs)
    pub shebang_options: ParseShebangOptions,
}

impl<'a> ExecResolveConfig<'a> {
    /// Configuration for execve - no PATH search, direct execution
    pub fn search_path_disabled() -> Self {
        Self {
            search_path: None,
            shebang_options: Default::default(),
        }
    }
    /// execlp/execvp/execvP/execvpe
    /// `custom_path` allows a customized path to be searched like in execvP (macOS extension)
    pub fn search_path_enabled(custom_path: Option<&'a BStr>) -> Self {
        Self {
            search_path: Some(SearchPath { custom_path }),
            shebang_options: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct Exec {
    pub program: BString,
    pub args: Vec<BString>,
    /// vec of (name, value). value is None when the entry in environ doesn't contain a `=` character.
    pub envs: Vec<(BString, Option<BString>)>,
}

fn getenv(name: &CStr) -> Option<&'static CStr> {
    let value = unsafe { nix::libc::getenv(name.as_ptr().cast()) };
    if value.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(value) })
    }
}

fn peek_executable(path: &Path, buf: &mut [u8]) -> nix::Result<usize> {
    let fd = open_executable(path)?;
    let mut total_read_size = 0;
    loop {
        let read_size = nix::unistd::read(&fd, &mut buf[total_read_size..])?;
        if read_size == 0 {
            break;
        }
        total_read_size += read_size;
    }
    Ok(total_read_size)
}

impl Exec {
    /// Resolve the program path according to exec family semantics
    ///
    /// This method replicates the behavior of execve/execvp/execvP/execvpe for program resolution,
    /// including PATH searching and shebang handling.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if resolution succeeds and `self` is updated with resolved paths
    /// * `Err(nix::Error)` with appropriate errno, like the exec function would return
    pub fn resolve(
        &mut self,
        mut on_path_access: impl FnMut(PathAccess<'_>),
        config: ExecResolveConfig,
    ) -> nix::Result<()> {
        if let Some(search_path) = config.search_path {
            let path = if let Some(custom_path) = search_path.custom_path {
                custom_path
            } else if let Some(path) = getenv(c"PATH") {
                path.to_bytes().as_bstr()
            } else {
                // https://github.com/kraj/musl/blob/1b06420abdf46f7d06ab4067e7c51b8b63731852/src/process/execvp.c#L21
                b"/usr/local/bin:/bin:/usr/bin".as_bstr()
            };
            let program = which::which(
                self.program.as_ref(),
                path,
                |path| {
                    on_path_access(PathAccess {
                        path: path.into(),
                        mode: AccessMode::Read,
                    });
                    access(OsStr::from_bytes(path), AccessFlags::X_OK)
                },
                |program| Ok(program.to_owned()),
            )?;
            self.program = program;
        }

        self.parse_shebang(on_path_access, config.shebang_options)?;

        Ok(())
    }

    fn parse_shebang(
        &mut self,
        mut on_path_access: impl FnMut(PathAccess<'_>),
        options: ParseShebangOptions,
    ) -> nix::Result<()> {
        if let Some(shebang) = parse_shebang(
            |path, buf| {
                on_path_access(PathAccess::read(path));
                peek_executable(path, buf)
            },
            Path::new(OsStr::from_bytes(&self.program)),
            options,
        )? {
            self.args[0] = shebang.interpreter.clone();
            let old_program = replace(&mut self.program, shebang.interpreter);
            self.args
                .splice(1..1, shebang.arguments.into_iter().chain(once(old_program)));
        }
        Ok(())
    }
}

pub fn ensure_env(
    envs: &mut Vec<(BString, Option<BString>)>,
    name: impl AsRef<BStr>,
    value: impl AsRef<BStr>,
) -> nix::Result<()> {
    let name = name.as_ref();
    let value = value.as_ref();
    let existing_value = envs
        .iter()
        .find_map(|(n, v)| if n == name { v.as_ref() } else { None });
    if let Some(existing_value) = existing_value {
        return if existing_value == value {
            Ok(())
        } else {
            Err(nix::Error::EINVAL)
        };
    };
    envs.push((name.to_owned(), Some(value.to_owned())));
    Ok(())
}
